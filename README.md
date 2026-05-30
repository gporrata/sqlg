# sqlg

SQL desktop POC built with Electron, React, CodeMirror, and Node database drivers.

## Features

- Electron main process owns database access with `pg` and `mysql2`.
- React renderer with three tabs: Connections, SQL Editor, and Results.
- CodeMirror SQL editor with syntax highlighting.
- `Ctrl+E` / `Command+E` executes the selected SQL, or the whole editor when nothing is selected.
- Query results are grouped by the first `id`-like column, with joined table rows shown underneath when columns are aliased with `prefix__column`.
- Local connection/settings storage is a JSON file under Electron's `userData` path for this POC.
- MySQL and Postgres sample databases are included in Docker Compose.

## Install

```sh
yarn install
```

## Run

```sh
yarn electron:dev
```

## Build Renderer

```sh
yarn build
```

## Typecheck

```sh
yarn run check
```

## Sample Databases

```sh
docker compose up -d
```

Default connections:

| Engine | Host | Port | Database | User | Password |
| --- | --- | ---: | --- | --- | --- |
| Postgres | `127.0.0.1` | `55432` | `sqlg` | `sqlg` | `sqlg` |
| MySQL | `127.0.0.1` | `33306` | `sqlg` | `root` | `sqlg` |

The editor starts with a query that demonstrates hierarchical results:

```sql
select
  c.id,
  c.name,
  c.email,
  o.id as orders__id,
  o.order_number as orders__order_number,
  o.total as orders__total,
  oi.id as items__id,
  oi.sku as items__sku,
  oi.quantity as items__quantity
from customers c
left join orders o on o.customer_id = c.id
left join order_items oi on oi.order_id = o.id
order by c.id, o.id, oi.id;
```

Reset sample database state:

```sh
docker compose down -v
docker compose up -d
```
