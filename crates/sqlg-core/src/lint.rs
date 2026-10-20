use std::collections::HashMap;
use std::ops::ControlFlow;

use serde::{Deserialize, Serialize};
use sqlparser::ast::{
    Expr, GroupByExpr, ObjectName, OrderBy, Query, Select, SelectItem, SetExpr, Spanned, Statement,
    TableFactor, TableWithJoins, Visit, Visitor,
};
use sqlparser::dialect::MySqlDialect;
use sqlparser::parser::Parser;
use sqlparser::tokenizer::{Location, Span};

use crate::split::{SqlStatement, split_statements};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub from: usize,
    pub to: usize,
    pub severity: String,
    pub message: String,
    pub actions: Vec<LintAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LintAction {
    pub name: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LintCatalog {
    #[serde(default)]
    pub default_schema: Option<String>,
    #[serde(default)]
    pub lower_case_table_names: u8,
    #[serde(default)]
    pub schemas: Vec<LintSchema>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LintSchema {
    pub name: String,
    pub tables: Vec<LintTable>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LintTable {
    pub name: String,
    pub columns: Vec<String>,
}

struct Resolved {
    table_name: String,
    columns: Vec<String>,
}

struct KnownTable {
    /// Alias or table name used as a qualifier, compared case-insensitively.
    alias: String,
    display: String,
    columns: Option<Vec<String>>,
    resolved: Option<Resolved>,
}

struct Scope {
    tables: Vec<KnownTable>,
}

/// Syntax, schema, and advisory warnings. Findings never block execution.
pub fn lint_sql(sql: &str, catalog: &LintCatalog) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for statement in split_statements(sql) {
        let mut found = lint_one(&statement, catalog);
        for diag in &mut found {
            diag.from += statement.start;
            diag.to += statement.start;
        }
        out.extend(found);
    }
    out
}

fn lint_one(statement: &SqlStatement, catalog: &LintCatalog) -> Vec<Diagnostic> {
    let dialect = MySqlDialect {};
    let mut diags = Vec::new();
    let parsed = match Parser::parse_sql(&dialect, &statement.sql) {
        Ok(statements) => statements,
        Err(err) => {
            diags.push(syntax_diag(&statement.sql, &err.to_string()));
            return diags;
        }
    };
    if parsed.len() != 1 {
        // The splitter already isolated one statement. A second parse result
        // means the parser disagreed; keep going with the first.
    }
    let Some(ast) = parsed.first() else {
        return diags;
    };
    keyword_warnings(&statement.sql, &mut diags);
    if catalog.schemas.is_empty() {
        structural_warnings(ast, &statement.sql, &mut diags);
        return diags;
    }
    structural_warnings(ast, &statement.sql, &mut diags);
    schema_checks(ast, &statement.sql, catalog, &mut diags);
    diags
}

fn syntax_diag(sql: &str, message: &str) -> Diagnostic {
    let (from, to) = if let Some((line, col)) = line_col(message) {
        let at = offset_at(sql, line, col);
        (at, (at + 1).min(sql.len().max(at)))
    } else {
        (0, sql.len().min(1).max(if sql.is_empty() { 0 } else { 1 }))
    };
    let to = if to <= from {
        (from + 1).min(sql.len().max(from + 1))
    } else {
        to
    };
    Diagnostic {
        from,
        to: to.min(sql.len().max(to)),
        severity: "error".into(),
        message: format!("Syntax: {message}"),
        actions: Vec::new(),
    }
}

fn line_col(message: &str) -> Option<(u64, u64)> {
    let line_at = message.find("Line:")?;
    let line = message[line_at + 5..]
        .trim_start()
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    let col_at = message.find("Column:")?;
    let col = message[col_at + 7..]
        .trim_start()
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()?;
    Some((line, col))
}

fn offset_at(input: &str, line: u64, column: u64) -> usize {
    if line == 0 {
        return 0;
    }
    let mut current = 1u64;
    if line == 1 {
        return advance(input, 0, column);
    }
    for (i, ch) in input.char_indices() {
        if ch == '\n' {
            current += 1;
            if current == line {
                return advance(input, i + 1, column);
            }
        }
    }
    input.len()
}

fn advance(input: &str, base: usize, column: u64) -> usize {
    let mut left = column.saturating_sub(1) as usize;
    let mut idx = base;
    for ch in input.get(base..).unwrap_or_default().chars() {
        if left == 0 {
            break;
        }
        idx += ch.len_utf8();
        left -= 1;
    }
    idx.min(input.len())
}

fn span_range(sql: &str, span: Span) -> (usize, usize) {
    if span.start == Location::empty() || span.start.line == 0 {
        return (0, sql.len().min(1));
    }
    let start = offset_at(sql, span.start.line, span.start.column);
    // sqlparser span ends are exclusive: a one-character token at column 10 ends at column 11.
    let mut end = offset_at(sql, span.end.line, span.end.column);
    if end <= start {
        end = (start + 1).min(sql.len());
    }
    (
        start.min(sql.len()),
        end.min(sql.len()).max(start.min(sql.len())),
    )
}

fn keyword_warnings(sql: &str, diags: &mut Vec<Diagnostic>) {
    if let Some(range) = find_outside_strings(sql, "= NULL") {
        diags.push(warn(
            range,
            "Comparing to NULL with = is unknown; use IS NULL.",
        ));
    }
    if find_keyword(sql, "ORDER").is_none()
        && let Some(range) = find_keyword(sql, "LIMIT")
    {
        diags.push(warn(
            range,
            "LIMIT without ORDER BY can return rows in any order.",
        ));
    }
}

fn structural_warnings(stmt: &Statement, sql: &str, diags: &mut Vec<Diagnostic>) {
    match stmt {
        Statement::Update(update) if update.selection.is_none() => {
            let name = table_display(&update.table);
            let range = span_range(sql, update.table.relation.span());
            diags.push(warn(
                range,
                &format!("UPDATE without WHERE will modify every row in {name}."),
            ));
        }
        Statement::Delete(delete) if delete.selection.is_none() => {
            let name = delete_target(delete);
            diags.push(warn(
                (0, name.len().min(sql.len()).max(1)),
                &format!("DELETE without WHERE will modify every row in {name}."),
            ));
        }
        Statement::Query(query) => warn_star(query, sql, diags),
        _ => {}
    }
}

fn warn_star(query: &Query, sql: &str, diags: &mut Vec<Diagnostic>) {
    let Some(select) = select_body(query) else {
        return;
    };
    for item in &select.projection {
        if matches!(item, SelectItem::Wildcard(_)) {
            diags.push(warn(
                (0, 6.min(sql.len())),
                "SELECT * returns every column.",
            ));
        }
    }
}

fn select_body(query: &Query) -> Option<&Select> {
    match query.body.as_ref() {
        SetExpr::Select(select) => Some(select),
        _ => None,
    }
}

fn schema_checks(stmt: &Statement, sql: &str, catalog: &LintCatalog, diags: &mut Vec<Diagnostic>) {
    let index = CatalogIndex::build(catalog);
    let mut scope = Scope { tables: Vec::new() };
    match stmt {
        Statement::Query(query) => {
            collect_ctes(query, &index, &mut scope);
            if let Some(select) = select_body(query) {
                for twj in &select.from {
                    collect_twj(twj, &index, catalog, sql, &mut scope, diags);
                }
            }
            check_exprs(stmt, sql, &index, &scope, diags);
        }
        Statement::Update(update) => {
            collect_twj(&update.table, &index, catalog, sql, &mut scope, diags);
            check_exprs(stmt, sql, &index, &scope, diags);
        }
        Statement::Delete(delete) => {
            let tables = match &delete.from {
                sqlparser::ast::FromTable::WithFromKeyword(tables)
                | sqlparser::ast::FromTable::WithoutKeyword(tables) => tables,
            };
            for twj in tables {
                collect_twj(twj, &index, catalog, sql, &mut scope, diags);
            }
            check_exprs(stmt, sql, &index, &scope, diags);
        }
        _ => {}
    }
}

fn collect_ctes(query: &Query, index: &CatalogIndex, scope: &mut Scope) {
    let Some(with) = &query.with else {
        return;
    };
    for cte in &with.cte_tables {
        let columns = if cte.alias.columns.is_empty() {
            columns_of_query(&cte.query)
        } else {
            Some(
                cte.alias
                    .columns
                    .iter()
                    .map(|col| col.name.to_string())
                    .collect(),
            )
        };
        scope.tables.push(KnownTable {
            alias: cte.alias.name.value.to_ascii_lowercase(),
            display: cte.alias.name.value.clone(),
            columns,
            resolved: None,
        });
        let _ = index;
    }
}

/// Names a select list introduces with `AS` or an implicit alias.
/// MySQL accepts those names in `GROUP BY`, `HAVING`, and `ORDER BY`.
fn select_output_aliases(select: &Select) -> Vec<String> {
    select
        .projection
        .iter()
        .filter_map(|item| match item {
            SelectItem::ExprWithAlias { alias, .. } => Some(alias.value.clone()),
            _ => None,
        })
        .collect()
}

fn columns_of_query(query: &Query) -> Option<Vec<String>> {
    let select = select_body(query)?;
    let mut cols = Vec::new();
    for item in &select.projection {
        match item {
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _) => return None,
            SelectItem::ExprWithAlias { alias, .. } => cols.push(alias.value.clone()),
            SelectItem::UnnamedExpr(Expr::Identifier(ident)) => cols.push(ident.value.clone()),
            SelectItem::UnnamedExpr(Expr::CompoundIdentifier(parts)) => {
                if let Some(last) = parts.last() {
                    cols.push(last.value.clone());
                }
            }
            _ => {}
        }
    }
    Some(cols)
}

fn collect_twj(
    twj: &TableWithJoins,
    index: &CatalogIndex,
    catalog: &LintCatalog,
    sql: &str,
    scope: &mut Scope,
    diags: &mut Vec<Diagnostic>,
) {
    collect_factor(&twj.relation, index, catalog, sql, scope, diags);
    for join in &twj.joins {
        collect_factor(&join.relation, index, catalog, sql, scope, diags);
    }
}

fn collect_factor(
    factor: &TableFactor,
    index: &CatalogIndex,
    catalog: &LintCatalog,
    sql: &str,
    scope: &mut Scope,
    diags: &mut Vec<Diagnostic>,
) {
    match factor {
        TableFactor::Table { name, alias, .. } => {
            let idents = object_idents(name);
            if idents.is_empty() {
                return;
            }
            let alias_name = alias
                .as_ref()
                .map(|a| a.name.value.clone())
                .unwrap_or_else(|| idents.last().map(|i| i.value.clone()).unwrap_or_default());
            if scope.tables.iter().any(|t| {
                t.alias == alias_name.to_ascii_lowercase()
                    && t.resolved.is_none()
                    && t.columns.is_some()
            }) {
                // Prefer a CTE of the same name over a catalog table.
                return;
            }
            match resolve_table(&idents, index, catalog) {
                TableLookup::Ok(resolved) => {
                    let columns = resolved.columns.clone();
                    scope.tables.push(KnownTable {
                        alias: alias_name.to_ascii_lowercase(),
                        display: resolved.table_name.clone(),
                        columns: Some(columns),
                        resolved: Some(resolved),
                    });
                }
                TableLookup::MissingSchema(schema_ident) => {
                    let range = span_range(sql, schema_ident.span);
                    diags.push(error(
                        range,
                        &format!("Unknown schema '{}'.", schema_ident.value),
                        &[],
                    ));
                }
                TableLookup::MissingTable(table_ident, schema) => {
                    let range = span_range(sql, table_ident.span);
                    let suggestions = index.suggest_tables(
                        &schema,
                        &table_ident.value,
                        catalog.lower_case_table_names,
                    );
                    let message = suggestion_message(
                        "table",
                        &table_ident.value,
                        Some(&schema),
                        &suggestions,
                    );
                    diags.push(error(range, &message, &suggestions));
                }
            }
        }
        TableFactor::Derived {
            subquery, alias, ..
        } => {
            let Some(alias) = alias else {
                return;
            };
            scope.tables.push(KnownTable {
                alias: alias.name.value.to_ascii_lowercase(),
                display: alias.name.value.clone(),
                columns: if alias.columns.is_empty() {
                    columns_of_query(subquery)
                } else {
                    Some(alias.columns.iter().map(|c| c.name.to_string()).collect())
                },
                resolved: None,
            });
        }
        TableFactor::NestedJoin {
            table_with_joins, ..
        } => {
            collect_twj(table_with_joins, index, catalog, sql, scope, diags);
        }
        _ => {}
    }
}

enum TableLookup<'a> {
    Ok(Resolved),
    MissingSchema(&'a sqlparser::ast::Ident),
    MissingTable(&'a sqlparser::ast::Ident, String),
}

fn resolve_table<'a>(
    idents: &[&'a sqlparser::ast::Ident],
    index: &CatalogIndex,
    catalog: &LintCatalog,
) -> TableLookup<'a> {
    let (schema, table) = if idents.len() >= 2 {
        (
            idents[idents.len() - 2].value.clone(),
            idents[idents.len() - 1],
        )
    } else {
        let Some(schema) = catalog
            .default_schema
            .clone()
            .or_else(|| index.only_schema())
        else {
            return TableLookup::MissingTable(idents[0], String::new());
        };
        (schema, idents[0])
    };
    if index.schema(&schema).is_none() {
        let schema_ident = if idents.len() >= 2 {
            idents[idents.len() - 2]
        } else {
            return TableLookup::MissingTable(table, schema);
        };
        return TableLookup::MissingSchema(schema_ident);
    }
    match index.table(&schema, &table.value, catalog.lower_case_table_names) {
        Some(found) => TableLookup::Ok(Resolved {
            table_name: found.name.clone(),
            columns: found.columns.clone(),
        }),
        None => TableLookup::MissingTable(table, schema),
    }
}

fn check_exprs(
    stmt: &Statement,
    sql: &str,
    index: &CatalogIndex,
    scope: &Scope,
    diags: &mut Vec<Diagnostic>,
) {
    let mut visitor = ColVisitor {
        sql,
        scope,
        index,
        diags,
        frames: Vec::new(),
    };
    let _ = stmt.visit(&mut visitor);
}

/// One query's select-list aliases, and whether the visitor is inside a
/// clause where MySQL can see them.
struct QueryFrame {
    aliases: Vec<String>,
    /// `GROUP BY`, `HAVING`, and this query's `ORDER BY` increment this.
    /// Nested queries push their own frame, so an outer alias stays hidden
    /// inside a subquery.
    alias_depth: u32,
    having: Option<*const Expr>,
    order_by: Option<*const OrderBy>,
    /// The body select has already supplied `aliases` and `having`.
    bound: bool,
}

struct ColVisitor<'a> {
    sql: &'a str,
    scope: &'a Scope,
    index: &'a CatalogIndex,
    diags: &'a mut Vec<Diagnostic>,
    frames: Vec<QueryFrame>,
}

impl Visitor for ColVisitor<'_> {
    type Break = ();

    fn pre_visit_query(&mut self, query: &Query) -> ControlFlow<Self::Break> {
        self.frames.push(QueryFrame {
            aliases: Vec::new(),
            alias_depth: 0,
            having: None,
            order_by: query.order_by.as_ref().map(std::ptr::from_ref),
            bound: false,
        });
        ControlFlow::Continue(())
    }

    fn post_visit_query(&mut self, _query: &Query) -> ControlFlow<Self::Break> {
        self.frames.pop();
        ControlFlow::Continue(())
    }

    fn pre_visit_select(&mut self, select: &Select) -> ControlFlow<Self::Break> {
        if let Some(frame) = self.frames.last_mut()
            && !frame.bound
        {
            frame.bound = true;
            frame.aliases = select_output_aliases(select);
            frame.having = select.having.as_ref().map(std::ptr::from_ref);
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_group_by(&mut self, _group_by: &GroupByExpr) -> ControlFlow<Self::Break> {
        self.enter_alias_clause();
        ControlFlow::Continue(())
    }

    fn post_visit_group_by(&mut self, _group_by: &GroupByExpr) -> ControlFlow<Self::Break> {
        self.leave_alias_clause();
        ControlFlow::Continue(())
    }

    fn pre_visit_order_by(&mut self, order_by: &OrderBy) -> ControlFlow<Self::Break> {
        if self.is_query_order_by(order_by) {
            self.enter_alias_clause();
        }
        ControlFlow::Continue(())
    }

    fn post_visit_order_by(&mut self, order_by: &OrderBy) -> ControlFlow<Self::Break> {
        if self.is_query_order_by(order_by) {
            self.leave_alias_clause();
        }
        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &Expr) -> ControlFlow<Self::Break> {
        if self.is_having_expr(expr) {
            self.enter_alias_clause();
        }
        match expr {
            Expr::Identifier(ident) => self.check_unqualified(ident),
            Expr::CompoundIdentifier(parts) if parts.len() >= 2 => self.check_compound(parts),
            _ => {}
        }
        ControlFlow::Continue(())
    }

    fn post_visit_expr(&mut self, expr: &Expr) -> ControlFlow<Self::Break> {
        if self.is_having_expr(expr) {
            self.leave_alias_clause();
        }
        ControlFlow::Continue(())
    }
}

impl ColVisitor<'_> {
    fn enter_alias_clause(&mut self) {
        if let Some(frame) = self.frames.last_mut() {
            frame.alias_depth += 1;
        }
    }

    fn leave_alias_clause(&mut self) {
        if let Some(frame) = self.frames.last_mut() {
            frame.alias_depth = frame.alias_depth.saturating_sub(1);
        }
    }

    fn is_having_expr(&self, expr: &Expr) -> bool {
        self.frames
            .last()
            .and_then(|frame| frame.having)
            .is_some_and(|having| std::ptr::eq(having, expr))
    }

    fn is_query_order_by(&self, order_by: &OrderBy) -> bool {
        self.frames
            .last()
            .and_then(|frame| frame.order_by)
            .is_some_and(|found| std::ptr::eq(found, order_by))
    }

    fn alias_in_scope(&self, name: &str) -> bool {
        self.frames.last().is_some_and(|frame| {
            frame.alias_depth > 0
                && frame
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(name))
        })
    }

    fn check_unqualified(&mut self, ident: &sqlparser::ast::Ident) {
        if self.scope.tables.is_empty() {
            return;
        }
        if self.alias_in_scope(&ident.value) {
            return;
        }
        let mut matches_known = false;
        let mut unknown_scope = false;
        let mut candidates = Vec::new();
        for table in &self.scope.tables {
            let Some(columns) = &table.columns else {
                unknown_scope = true;
                continue;
            };
            candidates.extend(columns.clone());
            if columns.iter().any(|c| c.eq_ignore_ascii_case(&ident.value)) {
                matches_known = true;
            }
        }
        if let Some(frame) = self.frames.last().filter(|frame| frame.alias_depth > 0) {
            candidates.extend(frame.aliases.iter().cloned());
        }
        if matches_known || unknown_scope {
            return;
        }
        let suggestions = suggest(&ident.value, &candidates);
        let range = span_range(self.sql, ident.span);
        let message = suggestion_message("column", &ident.value, None, &suggestions);
        self.diags.push(error(range, &message, &suggestions));
    }

    fn check_compound(&mut self, parts: &[sqlparser::ast::Ident]) {
        let column = parts.last().unwrap();
        let qualifier = &parts[parts.len() - 2];
        if let Some(table) = self
            .scope
            .tables
            .iter()
            .find(|t| t.alias.eq_ignore_ascii_case(&qualifier.value))
        {
            let Some(columns) = &table.columns else {
                return;
            };
            if columns
                .iter()
                .any(|c| c.eq_ignore_ascii_case(&column.value))
            {
                return;
            }
            let suggestions = suggest(&column.value, columns);
            let range = span_range(self.sql, column.span);
            let message =
                suggestion_message("column", &column.value, Some(&table.display), &suggestions);
            self.diags.push(error(range, &message, &suggestions));
            return;
        }
        if parts.len() >= 3 {
            let schema = &parts[parts.len() - 3];
            let table_ident = qualifier;
            if let Some(found) = self
                .index
                .table(schema.value.as_str(), &table_ident.value, 0)
                && !found
                    .columns
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(&column.value))
            {
                let suggestions = suggest(&column.value, &found.columns);
                let range = span_range(self.sql, column.span);
                let message =
                    suggestion_message("column", &column.value, Some(&found.name), &suggestions);
                self.diags.push(error(range, &message, &suggestions));
            }
        }
    }
}

fn suggestion_message(
    kind: &str,
    name: &str,
    owner: Option<&str>,
    suggestions: &[String],
) -> String {
    let where_ = owner.map(|o| format!(" in {o}")).unwrap_or_default();
    if let Some(best) = suggestions.first() {
        format!("Unknown {kind} '{name}'{where_}. Did you mean '{best}'?")
    } else {
        format!("Unknown {kind} '{name}'{where_}.")
    }
}

fn error(range: (usize, usize), message: &str, suggestions: &[String]) -> Diagnostic {
    Diagnostic {
        from: range.0,
        to: range.1.max(range.0),
        severity: "error".into(),
        message: message.into(),
        actions: suggestions
            .iter()
            .take(1)
            .map(|text| LintAction {
                name: format!("Replace with {text}"),
                text: text.clone(),
            })
            .collect(),
    }
}

fn warn(range: (usize, usize), message: &str) -> Diagnostic {
    Diagnostic {
        from: range.0,
        to: range.1.max(range.0 + 1),
        severity: "warning".into(),
        message: message.into(),
        actions: Vec::new(),
    }
}

fn object_idents(name: &ObjectName) -> Vec<&sqlparser::ast::Ident> {
    name.0.iter().filter_map(|part| part.as_ident()).collect()
}

fn table_display(twj: &TableWithJoins) -> String {
    match &twj.relation {
        TableFactor::Table { name, .. } => object_idents(name)
            .last()
            .map(|i| i.value.clone())
            .unwrap_or_else(|| "the table".into()),
        _ => "the table".into(),
    }
}

struct CatalogIndex {
    schemas: HashMap<String, HashMap<String, LintTable>>,
    display_schema: HashMap<String, String>,
}

impl CatalogIndex {
    fn build(catalog: &LintCatalog) -> Self {
        let mut schemas = HashMap::new();
        let mut display_schema = HashMap::new();
        for schema in &catalog.schemas {
            let key = schema.name.to_ascii_lowercase();
            display_schema.insert(key.clone(), schema.name.clone());
            let mut tables = HashMap::new();
            for table in &schema.tables {
                tables.insert(table.name.to_ascii_lowercase(), table.clone());
                if catalog.lower_case_table_names == 0 {
                    tables.insert(table.name.clone(), table.clone());
                }
            }
            schemas.insert(key, tables);
        }
        Self {
            schemas,
            display_schema,
        }
    }

    fn schema(&self, name: &str) -> Option<&HashMap<String, LintTable>> {
        self.schemas.get(&name.to_ascii_lowercase())
    }

    fn only_schema(&self) -> Option<String> {
        if self.display_schema.len() == 1 {
            self.display_schema.values().next().cloned()
        } else {
            None
        }
    }

    fn table(&self, schema: &str, table: &str, lower_case: u8) -> Option<&LintTable> {
        let tables = self.schema(schema)?;
        if lower_case == 0 {
            tables
                .get(table)
                .or_else(|| tables.get(&table.to_ascii_lowercase()))
        } else {
            tables.get(&table.to_ascii_lowercase())
        }
    }

    fn suggest_tables(&self, schema: &str, table: &str, _lower: u8) -> Vec<String> {
        let Some(tables) = self.schema(schema) else {
            return Vec::new();
        };
        let names: Vec<String> = tables.values().map(|t| t.name.clone()).collect();
        suggest(table, &names)
    }
}

fn suggest(target: &str, candidates: &[String]) -> Vec<String> {
    let mut scored: Vec<(usize, String)> = candidates
        .iter()
        .filter_map(|candidate| {
            let distance = damerau(
                &target.to_ascii_lowercase(),
                &candidate.to_ascii_lowercase(),
            );
            (distance > 0 && distance <= 2).then_some((distance, candidate.clone()))
        })
        .collect();
    scored.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.to_ascii_lowercase().cmp(&b.1.to_ascii_lowercase()))
    });
    scored.dedup_by(|a, b| a.1.eq_ignore_ascii_case(&b.1));
    scored.truncate(3);
    scored.into_iter().map(|(_, name)| name).collect()
}

fn damerau(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let n = a.len();
    let m = b.len();
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev = (0..=m).collect::<Vec<_>>();
    let mut curr = vec![0; m + 1];
    let mut prev2 = vec![0; m + 1];
    for i in 1..=n {
        curr[0] = i;
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                curr[j] = curr[j].min(prev2[j - 2] + 1);
            }
        }
        prev2.clone_from(&prev);
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[m]
}

fn find_keyword(sql: &str, keyword: &str) -> Option<(usize, usize)> {
    let lower = sql.to_ascii_lowercase();
    let needle = keyword.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut i = 0usize;
    while i + needle.len() <= bytes.len() {
        if bytes[i..].starts_with(needle.as_bytes()) {
            let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            let after = i + needle.len();
            let after_ok = after >= bytes.len() || !bytes[after].is_ascii_alphanumeric();
            if before_ok && after_ok && !inside_string_or_comment(sql, i) {
                return Some((i, after));
            }
        }
        i += 1;
    }
    None
}

fn find_outside_strings(sql: &str, phrase: &str) -> Option<(usize, usize)> {
    let lower = sql.to_ascii_lowercase();
    let needle = phrase.to_ascii_lowercase();
    let mut start = 0usize;
    while let Some(rel) = lower[start..].find(&needle) {
        let at = start + rel;
        if !inside_string_or_comment(sql, at) {
            return Some((at, at + phrase.len()));
        }
        start = at + 1;
    }
    None
}

fn inside_string_or_comment(sql: &str, at: usize) -> bool {
    let chars: Vec<(usize, char)> = sql.char_indices().collect();
    let mut mode = 0u8; // 0 normal, 1 sq, 2 dq, 3 bq, 4 line, 5 block
    let mut i = 0usize;
    while i < chars.len() && chars[i].0 < at {
        let ch = chars[i].1;
        let next = chars.get(i + 1).map(|(_, c)| *c);
        match mode {
            0 => match ch {
                '\'' => mode = 1,
                '"' => mode = 2,
                '`' => mode = 3,
                '#' => mode = 4,
                '-' if next == Some('-') => mode = 4,
                '/' if next == Some('*') => {
                    mode = 5;
                    i += 1;
                }
                _ => {}
            },
            1 if ch == '\'' => mode = 0,
            2 if ch == '"' => mode = 0,
            3 if ch == '`' => mode = 0,
            4 if ch == '\n' => mode = 0,
            5 if ch == '*' && next == Some('/') => {
                mode = 0;
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    mode != 0
}

fn delete_target(delete: &sqlparser::ast::Delete) -> String {
    delete
        .tables
        .first()
        .map(|name| name.to_string())
        .or_else(|| {
            let tables = match &delete.from {
                sqlparser::ast::FromTable::WithFromKeyword(tables)
                | sqlparser::ast::FromTable::WithoutKeyword(tables) => tables,
            };
            tables.first().map(table_display)
        })
        .unwrap_or_else(|| "the table".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fleet() -> LintCatalog {
        LintCatalog {
            default_schema: Some("fleet".into()),
            lower_case_table_names: 0,
            schemas: vec![LintSchema {
                name: "fleet".into(),
                tables: vec![
                    LintTable {
                        name: "DRIVER".into(),
                        columns: vec!["id".into(), "name".into(), "email".into()],
                    },
                    LintTable {
                        name: "TRIP".into(),
                        columns: vec!["id".into(), "driver_id".into(), "status".into()],
                    },
                ],
            }],
        }
    }

    #[test]
    fn alias_typo_suggests_email() {
        let sql = "SELECT d.emial FROM TRIP t JOIN DRIVER d ON d.id = t.driver_id";
        let diags = lint_sql(sql, &fleet());
        let diag = diags
            .iter()
            .find(|d| d.message.contains("emial"))
            .expect("diag");
        assert_eq!(diag.severity, "error");
        assert!(diag.message.contains("email"));
        assert_eq!(diag.actions[0].text, "email");
        assert_eq!(&sql[diag.from..diag.to], "emial");
    }

    #[test]
    fn update_without_where_warns_and_does_not_cascade_on_parse_error() {
        let sql = "UPDATE TRIP SET status = 'completed'";
        let diags = lint_sql(sql, &fleet());
        assert!(
            diags
                .iter()
                .any(|d| d.severity == "warning" && d.message.contains("UPDATE"))
        );

        let broken = "SELEC FROM";
        let diags = lint_sql(broken, &fleet());
        assert!(diags.iter().all(|d| d.message.starts_with("Syntax")));
    }

    #[test]
    fn parser_error_has_a_position() {
        let sql = "SELECT FROM TRIP";
        let diags = lint_sql(sql, &LintCatalog::default());
        assert!(!diags.is_empty());
        assert!(diags[0].to >= diags[0].from);
    }

    fn book() -> LintCatalog {
        LintCatalog {
            default_schema: Some("book".into()),
            lower_case_table_names: 0,
            schemas: vec![LintSchema {
                name: "book".into(),
                tables: vec![
                    LintTable {
                        name: "TRIP".into(),
                        columns: vec!["id".into()],
                    },
                    LintTable {
                        name: "TRAVEL_BOOKING".into(),
                        columns: vec!["id".into(), "trip_id".into()],
                    },
                ],
            }],
        }
    }

    #[test]
    fn having_accepts_a_select_alias() {
        let sql = "-- find a trip with multiple v2 manual booking notes... and possible trip note
select t.id, count(tb.id) tbcnt
from book.TRIP t
join book.TRAVEL_BOOKING tb on tb.trip_id = t.id
group by t.id
having tbcnt > 1";
        let diags = lint_sql(sql, &book());
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn group_by_and_order_by_accept_a_select_alias() {
        let sql = "SELECT t.status AS state, COUNT(t.id) AS trips FROM TRIP t GROUP BY state ORDER BY trips";
        let diags = lint_sql(sql, &fleet());
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn where_still_rejects_a_select_alias() {
        let sql = "SELECT t.status AS state FROM TRIP t WHERE state = 'completed'";
        let diags = lint_sql(sql, &fleet());
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("state"));
        assert!(diags[0].from > sql.find("AS state").unwrap());
    }

    #[test]
    fn having_alias_does_not_hide_a_nested_or_window_reference() {
        let nested =
            "SELECT COUNT(t.id) AS tbcnt FROM TRIP t HAVING tbcnt > (SELECT tbcnt FROM DRIVER)";
        let diags = lint_sql(nested, &fleet());
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(&nested[diags[0].from..diags[0].to], "tbcnt");
        assert!(diags[0].from > nested.find("SELECT tbcnt").unwrap());

        let window = "SELECT COUNT(t.id) AS tbcnt, SUM(t.id) OVER (ORDER BY tbcnt) FROM TRIP t";
        let diags = lint_sql(window, &fleet());
        assert!(
            diags.iter().any(|diag| diag.message.contains("tbcnt")),
            "{diags:?}"
        );
    }

    #[test]
    fn having_typo_suggests_the_select_alias() {
        let sql = "SELECT t.status AS state FROM TRIP t GROUP BY state HAVING sttae = 'completed'";
        let diags = lint_sql(sql, &fleet());
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert!(diags[0].message.contains("state"), "{}", diags[0].message);
        assert_eq!(&sql[diags[0].from..diags[0].to], "sttae");
    }

    #[test]
    fn distance_catches_a_transposition() {
        assert!(damerau("emial", "email") <= 2);
        assert!(damerau("status", "zzzzzz") > 2);
    }
}
