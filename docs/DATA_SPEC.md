# MEVDAN — Data Specification

`mevdan-data` handles structured data: CSV, JSON, JSONL.

## Formats

| Format | Loading |
|--------|---------|
| CSV | ✅ |
| JSON | ✅ |
| JSONL | ✅ |
| Parquet | ❌ (V5.5+) |
| SQLite | ❌ (V5.5+) |

## Concepts

### DataValue

A typed value:

```rust
pub enum DataValue {
    Null,
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
}
Accessors: as_int, as_float (accepts Int), as_text, as_bool.

to_display_string() — for printing.

ColumnType
rust
pub enum ColumnType {
    Int, Float, Text, Bool, Unknown,
}
Helpers:

infer(s) — infer from text.

combine(other) — combine two types for a column
(Int + Float = Float, mixed = Text).

DataColumn
rust
pub struct DataColumn {
    pub name: String,
    pub ty: ColumnType,
}
DataTable
In-memory table with columns and rows.

rust
let mut t = DataTable::new(vec![
    DataColumn::new("name", ColumnType::Text),
    DataColumn::new("age", ColumnType::Int),
]);
t.push_row(vec![
    DataValue::Text("Alice".into()),
    DataValue::Int(30),
])?;
Operations:

push_row(row) — validate length against columns.

column_names(), columns(), column_count(), row_count().

column_index(name) — name → index.

row(i), get(i, column_name).

filter(|row| ...) — new table with matching rows.

select(&["name", "age"]) — new table with selected columns.

head(n) — first n rows.

add_computed_column(name, type, f) — append computed column.

summary() — textual summary.

Note: column_index is rebuilt on deserialization.

DataQuery
Chainable filters + projection + ordering + limit.

rust
let q = DataQuery::new()
    .where_eq("city", DataValue::Text("Madrid".into()))
    .where_gt("age", DataValue::Int(28))
    .order_by("age", false)
    .limit(10)
    .select(vec!["name".into(), "age".into()]);

let result = q.execute(&table)?;
CompareOp: Eq, Neq, Lt, Lte, Gt, Gte, Contains.

LogicalOp: And (default), Or.

DataLoader
rust
let loader = DataLoader::new();
let table = loader.load("people.csv")?;
Format specifics
CSV
First row is the header.

Types are inferred per column by scanning all values.

No quoted field support (V5.4).

JSON
Expected: array of objects.

Column set = union of keys.

Missing keys → Null.

JSONL
One JSON object per line.

Blank lines ignored.

Same rules as JSON array.

What this is NOT
No SQL. Filters are simple.

No joins. Single-table operations only.

No streaming. Whole file loaded into memory.

No Parquet/SQLite. Recognized but not loaded.

No quoted CSV. Only comma splitting.

Design notes
Typed values. Ints and floats are distinguished.

Column type inference. Automatic from data.

Serializable. DataTable roundtrips through JSON.

Composable. filter/select/head return new tables.
