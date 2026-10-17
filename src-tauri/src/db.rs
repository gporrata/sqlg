use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use futures::StreamExt;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Serialize;
use sqlg_core::{Connection, split_statements};
use sqlx::mysql::{MySqlPool, MySqlPoolOptions, MySqlRow};
use sqlx::{Column, Executor, Row, TypeInfo};
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnInfo {
    pub name: String,
    pub type_name: String,
    pub numeric: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaColumn {
    pub name: String,
    pub data_type: String,
    pub column_type: String,
    pub column_key: String,
    pub foreign_key: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaTable {
    pub name: String,
    pub columns: Vec<SchemaColumn>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaGroup {
    pub name: String,
    pub tables: Vec<SchemaTable>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaSnapshot {
    pub lower_case_table_names: u8,
    pub schemas: Vec<SchemaGroup>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum QueryEvent {
    Begin {
        run_id: String,
        index: usize,
        total: usize,
        sql: String,
    },
    Columns {
        index: usize,
        columns: Vec<ColumnInfo>,
    },
    Rows {
        index: usize,
        rows: Vec<Vec<Option<String>>>,
    },
    StatementDone {
        index: usize,
        rows_affected: Option<u64>,
        row_count: u64,
        elapsed_ms: u64,
    },
    StatementError {
        index: usize,
        message: String,
    },
    Finished {
        ok: bool,
        executed: usize,
        total: usize,
        failed: usize,
        elapsed_ms: u64,
        cancelled: bool,
    },
}

pub fn mysql_url(connection: &Connection, password: &str) -> String {
    let user = utf8_percent_encode(&connection.user, NON_ALPHANUMERIC);
    let pass = utf8_percent_encode(password, NON_ALPHANUMERIC);
    let host = host_for_url(&connection.host);
    let db = connection
        .schema
        .as_deref()
        .map(|schema| utf8_percent_encode(schema, NON_ALPHANUMERIC).to_string())
        .unwrap_or_default();
    let ssl = if connection.ssl {
        "REQUIRED"
    } else {
        "DISABLED"
    };
    format!(
        "mysql://{user}:{pass}@{host}:{}/{db}?ssl-mode={ssl}",
        connection.port
    )
}

fn host_for_url(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

pub fn format_mysql_message(number: u16, state: &str, message: &str) -> String {
    format!("Error {number} ({state}): {message}")
}

pub fn format_sqlx_error(err: &sqlx::Error) -> String {
    if let sqlx::Error::Database(db) = err {
        if let Some(mysql) = db.try_downcast_ref::<sqlx::mysql::MySqlDatabaseError>() {
            let state = mysql.code().unwrap_or("");
            return format_mysql_message(mysql.number(), state, mysql.message());
        }
        return format!("Error: {db}");
    }
    err.to_string()
}

pub async fn connect_pool(connection: &Connection, password: &str) -> Result<MySqlPool, String> {
    MySqlPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(8))
        .connect(&mysql_url(connection, password))
        .await
        .map_err(|err| format_sqlx_error(&err))
}

pub async fn server_version(pool: &MySqlPool) -> Result<String, String> {
    sqlx::query_scalar::<_, String>("select version()")
        .fetch_one(pool)
        .await
        .map_err(|err| format_sqlx_error(&err))
}

pub async fn load_schema(pool: &MySqlPool) -> Result<SchemaSnapshot, String> {
    let lower: u64 = sqlx::query_scalar("select @@lower_case_table_names")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let column_rows = sqlx::query(
        "select table_schema, table_name, column_name, data_type, column_type, column_key
         from information_schema.columns
         where table_schema not in ('information_schema', 'mysql', 'performance_schema', 'sys')
         order by table_schema, table_name, ordinal_position",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| format_sqlx_error(&err))?;
    let fk_rows = sqlx::query(
        "select table_schema, table_name, column_name
         from information_schema.key_column_usage
         where referenced_table_name is not null
           and table_schema not in ('information_schema', 'mysql', 'performance_schema', 'sys')",
    )
    .fetch_all(pool)
    .await
    .map_err(|err| format_sqlx_error(&err))?;

    let mut foreign = BTreeSet::new();
    for row in &fk_rows {
        let schema: String = row.try_get(0).unwrap_or_default();
        let table: String = row.try_get(1).unwrap_or_default();
        let column: String = row.try_get(2).unwrap_or_default();
        foreign.insert((schema, table, column));
    }

    let mut schemas: BTreeMap<String, BTreeMap<String, Vec<SchemaColumn>>> = BTreeMap::new();
    for row in &column_rows {
        let schema: String = row.try_get(0).unwrap_or_default();
        let table: String = row.try_get(1).unwrap_or_default();
        let name: String = row.try_get(2).unwrap_or_default();
        let data_type: String = row.try_get(3).unwrap_or_default();
        let column_type: String = row.try_get(4).unwrap_or_default();
        let column_key: String = row.try_get(5).unwrap_or_default();
        let foreign_key = foreign.contains(&(schema.clone(), table.clone(), name.clone()));
        schemas
            .entry(schema)
            .or_default()
            .entry(table)
            .or_default()
            .push(SchemaColumn {
                name,
                data_type,
                column_type,
                column_key,
                foreign_key,
            });
    }

    let schemas = schemas
        .into_iter()
        .map(|(name, tables)| SchemaGroup {
            name,
            tables: tables
                .into_iter()
                .map(|(name, columns)| SchemaTable { name, columns })
                .collect(),
        })
        .collect();
    Ok(SchemaSnapshot {
        lower_case_table_names: lower.min(255) as u8,
        schemas,
    })
}

pub async fn explain_select(pool: &MySqlPool, sql: &str) -> Result<String, String> {
    if !is_single_select(sql) {
        return Err("explain is limited to a single select".into());
    }
    let statement = format!("explain {sql}");
    let fut = sqlx::query(&statement).fetch_all(pool);
    let rows = tokio::time::timeout(Duration::from_secs(1), fut)
        .await
        .map_err(|_| "explain timed out".to_string())?
        .map_err(|err| format_sqlx_error(&err))?;
    if rows.is_empty() {
        return Ok("explain returned no rows".into());
    }
    let mut lines = Vec::new();
    for row in &rows {
        let mut cells = Vec::new();
        for (idx, col) in row.columns().iter().enumerate() {
            let value = cell_value(row, idx).unwrap_or_else(|| "null".into());
            cells.push(format!("{}={value}", col.name()));
        }
        lines.push(cells.join(" "));
    }
    Ok(lines.join("\n"))
}

pub fn is_single_select(sql: &str) -> bool {
    let parts = split_statements(sql);
    if parts.len() != 1 {
        return false;
    }
    let dialect = sqlparser::dialect::MySqlDialect {};
    let Ok(parsed) = sqlparser::parser::Parser::parse_sql(&dialect, &parts[0].sql) else {
        return false;
    };
    parsed.len() == 1 && matches!(parsed.first(), Some(sqlparser::ast::Statement::Query(_)))
}

pub async fn execute_script(
    pool: MySqlPool,
    run_id: String,
    sql: String,
    cancel: Arc<AtomicBool>,
    connection_id: Arc<AtomicU64>,
    channel: Channel<QueryEvent>,
) {
    let started = Instant::now();
    let statements = split_statements(&sql);
    let total = statements.len();
    if total == 0 {
        let _ = channel.send(QueryEvent::Finished {
            ok: true,
            executed: 0,
            total: 0,
            failed: 0,
            elapsed_ms: 0,
            cancelled: false,
        });
        return;
    }
    let mut conn = match pool.acquire().await {
        Ok(conn) => conn,
        Err(err) => {
            let _ = channel.send(QueryEvent::StatementError {
                index: 0,
                message: format_sqlx_error(&err),
            });
            let _ = channel.send(QueryEvent::Finished {
                ok: false,
                executed: 1,
                total,
                failed: 1,
                elapsed_ms: elapsed(started),
                cancelled: false,
            });
            return;
        }
    };
    if let Ok(id) = sqlx::query_scalar::<_, u64>("select connection_id()")
        .fetch_one(&mut *conn)
        .await
    {
        connection_id.store(id, Ordering::SeqCst);
    }

    let mut failed = 0usize;
    let mut executed = 0usize;
    let mut cancelled = false;
    for (index, statement) in statements.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            cancelled = true;
            break;
        }
        let _ = channel.send(QueryEvent::Begin {
            run_id: run_id.clone(),
            index,
            total,
            sql: statement.sql.clone(),
        });
        let one_started = Instant::now();
        match run_statement(
            &mut conn,
            &pool,
            &statement.sql,
            index,
            &cancel,
            &connection_id,
            &channel,
        )
        .await
        {
            Ok(()) => {
                executed += 1;
                let _ = one_started;
            }
            Err(StatementStop::Failed(message)) => {
                executed += 1;
                failed += 1;
                let _ = channel.send(QueryEvent::StatementError { index, message });
                break;
            }
            Err(StatementStop::Cancelled) => {
                cancelled = true;
                break;
            }
        }
    }
    let _ = channel.send(QueryEvent::Finished {
        ok: failed == 0 && !cancelled,
        executed,
        total,
        failed,
        elapsed_ms: elapsed(started),
        cancelled,
    });
}

enum StatementStop {
    Failed(String),
    Cancelled,
}

async fn run_statement(
    conn: &mut sqlx::pool::PoolConnection<sqlx::MySql>,
    pool: &MySqlPool,
    sql: &str,
    index: usize,
    cancel: &AtomicBool,
    connection_id: &AtomicU64,
    channel: &Channel<QueryEvent>,
) -> Result<(), StatementStop> {
    let started = Instant::now();
    let mut stream = sqlx::raw_sql(sql).fetch_many(&mut **conn);
    let mut columns: Option<Vec<ColumnInfo>> = None;
    let mut batch = Vec::new();
    let mut row_count = 0u64;
    let mut rows_affected: Option<u64> = None;
    let mut saw_rows = false;

    while let Some(item) = stream.next().await {
        if cancel.load(Ordering::SeqCst) {
            kill_query(pool, connection_id.load(Ordering::SeqCst)).await;
            return Err(StatementStop::Cancelled);
        }
        match item {
            Ok(sqlx::Either::Left(result)) => {
                rows_affected = Some(result.rows_affected());
            }
            Ok(sqlx::Either::Right(row)) => {
                saw_rows = true;
                if columns.is_none() {
                    let info = columns_of(&row);
                    let _ = channel.send(QueryEvent::Columns {
                        index,
                        columns: info.clone(),
                    });
                    columns = Some(info);
                }
                batch.push(row_cells(&row));
                row_count += 1;
                if batch.len() >= 200 {
                    let _ = channel.send(QueryEvent::Rows {
                        index,
                        rows: std::mem::take(&mut batch),
                    });
                }
            }
            Err(err) => {
                if cancel.load(Ordering::SeqCst) {
                    return Err(StatementStop::Cancelled);
                }
                return Err(StatementStop::Failed(format_sqlx_error(&err)));
            }
        }
    }
    if !batch.is_empty() {
        let _ = channel.send(QueryEvent::Rows { index, rows: batch });
    }
    drop(stream);
    if !saw_rows
        && rows_affected.is_none()
        && let Ok(desc) = conn.describe(sql).await
    {
        let info: Vec<ColumnInfo> = desc
            .columns()
            .iter()
            .map(|col| {
                let type_name = col.type_info().name().to_string();
                ColumnInfo {
                    numeric: is_numeric(&type_name),
                    name: col.name().to_string(),
                    type_name,
                }
            })
            .collect();
        if !info.is_empty() {
            let _ = channel.send(QueryEvent::Columns {
                index,
                columns: info,
            });
        }
    }
    let _ = channel.send(QueryEvent::StatementDone {
        index,
        rows_affected: if saw_rows { None } else { rows_affected },
        row_count,
        elapsed_ms: elapsed(started),
    });
    Ok(())
}

async fn kill_query(pool: &MySqlPool, id: u64) {
    if id == 0 {
        return;
    }
    let statement = format!("kill query {id}");
    let _ = sqlx::query(&statement).execute(pool).await;
}

fn columns_of(row: &MySqlRow) -> Vec<ColumnInfo> {
    row.columns()
        .iter()
        .map(|col| {
            let type_name = col.type_info().name().to_string();
            ColumnInfo {
                numeric: is_numeric(&type_name),
                name: col.name().to_string(),
                type_name,
            }
        })
        .collect()
}

fn row_cells(row: &MySqlRow) -> Vec<Option<String>> {
    (0..row.columns().len())
        .map(|idx| cell_value(row, idx))
        .collect()
}

fn cell_value(row: &MySqlRow, idx: usize) -> Option<String> {
    if let Ok(value) = row.try_get::<Option<i64>, _>(idx) {
        return value.map(|number| number.to_string());
    }
    if let Ok(value) = row.try_get::<Option<u64>, _>(idx) {
        return value.map(|number| number.to_string());
    }
    if let Ok(value) = row.try_get::<Option<f64>, _>(idx) {
        return value.map(|number| number.to_string());
    }
    if let Ok(value) = row.try_get::<Option<String>, _>(idx) {
        return value;
    }
    if let Ok(value) = row.try_get::<Option<chrono::NaiveDateTime>, _>(idx) {
        return value.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());
    }
    if let Ok(value) = row.try_get::<Option<chrono::NaiveDate>, _>(idx) {
        return value.map(|date| date.format("%Y-%m-%d").to_string());
    }
    if let Ok(value) = row.try_get::<Option<Vec<u8>>, _>(idx) {
        return value.map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
    }
    None
}

fn is_numeric(type_name: &str) -> bool {
    let lower = type_name.to_ascii_lowercase();
    [
        "int", "decimal", "numeric", "float", "double", "bit", "real", "serial",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn elapsed(started: Instant) -> u64 {
    started.elapsed().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_encodes_the_password_and_wraps_ipv6() {
        let connection = Connection {
            name: "dev".into(),
            host: "::1".into(),
            port: 3306,
            user: "app user".into(),
            schema: Some("fleet".into()),
            ssl: true,
            explain_lint: false,
            color: None,
        };
        let url = mysql_url(&connection, "p@ss word");
        assert!(url.contains("app%20user"));
        assert!(url.contains("p%40ss%20word"));
        assert!(url.contains("[::1]:3306"));
        assert!(url.contains("ssl-mode=REQUIRED"));
        assert!(!url.contains("p@ss"));
    }

    #[test]
    fn mysql_error_shape() {
        assert_eq!(
            format_mysql_message(1146, "42S02", "Table 'fleet.TRIPS' doesn't exist"),
            "Error 1146 (42S02): Table 'fleet.TRIPS' doesn't exist"
        );
    }

    #[test]
    fn explain_rejects_updates() {
        assert!(!is_single_select("update trip set status = 'x'"));
        assert!(is_single_select("select 1"));
        assert!(!is_single_select("select 1; select 2"));
    }
}
