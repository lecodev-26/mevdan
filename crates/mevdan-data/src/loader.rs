//! `DataLoader` — carga datasets desde disco.

use crate::{
    error::{DataError, DataResult},
    table::DataTable,
    value::{ColumnType, DataColumn, DataValue},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Formato de un dataset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataFormat {
    Csv,
    Json,
    Jsonl,
    Parquet,
    Sqlite,
    Unknown,
}

impl DataFormat {
    pub fn display_name(&self) -> &'static str {
        match self {
            DataFormat::Csv => "csv",
            DataFormat::Json => "json",
            DataFormat::Jsonl => "jsonl",
            DataFormat::Parquet => "parquet",
            DataFormat::Sqlite => "sqlite",
            DataFormat::Unknown => "unknown",
        }
    }

    pub fn is_supported(&self) -> bool {
        matches!(self, DataFormat::Csv | DataFormat::Json | DataFormat::Jsonl)
    }

    pub fn from_path(path: &Path) -> DataResult<Self> {
        let ext = match path.extension().and_then(|s| s.to_str()) {
            Some(e) => e.to_lowercase(),
            None => return Err(DataError::UnsupportedFormat(path.display().to_string())),
        };
        Ok(match ext.as_str() {
            "csv" => DataFormat::Csv,
            "json" => DataFormat::Json,
            "jsonl" | "ndjson" => DataFormat::Jsonl,
            "parquet" => DataFormat::Parquet,
            "db" | "sqlite" | "sqlite3" => DataFormat::Sqlite,
            _ => DataFormat::Unknown,
        })
    }
}

/// Cargador de datasets.
#[derive(Debug, Default)]
pub struct DataLoader;

impl DataLoader {
    pub fn new() -> Self {
        Self
    }

    /// Carga un dataset desde una ruta.
    pub fn load(&self, path: impl AsRef<Path>) -> DataResult<DataTable> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(DataError::NotFound(path.display().to_string()));
        }
        if !path.is_file() {
            return Err(DataError::NotFound(path.display().to_string()));
        }

        let format = DataFormat::from_path(path)?;

        match format {
            DataFormat::Csv => self.load_csv(path),
            DataFormat::Json => self.load_json(path),
            DataFormat::Jsonl => self.load_jsonl(path),
            _ => Err(DataError::UnsupportedFormat(
                format.display_name().to_string(),
            )),
        }
    }

    fn load_csv(&self, path: &Path) -> DataResult<DataTable> {
        let text = fs::read_to_string(path)?;
        parse_csv(&text)
    }

    fn load_json(&self, path: &Path) -> DataResult<DataTable> {
        let text = fs::read_to_string(path)?;
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| DataError::ParseError(format!("invalid JSON: {}", e)))?;
        json_array_to_table(&value)
    }

    fn load_jsonl(&self, path: &Path) -> DataResult<DataTable> {
        let text = fs::read_to_string(path)?;
        let mut objects: Vec<serde_json::Value> = Vec::new();
        for (idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let v: serde_json::Value = serde_json::from_str(trimmed)
                .map_err(|e| DataError::ParseError(format!("line {}: {}", idx + 1, e)))?;
            objects.push(v);
        }
        json_array_to_table(&serde_json::Value::Array(objects))
    }
}

/// Parsea un CSV simple (sin escaping de comillas dobles por ahora).
fn parse_csv(text: &str) -> DataResult<DataTable> {
    let mut lines = text.lines();

    // Cabecera.
    let header_line = lines
        .next()
        .ok_or_else(|| DataError::EmptyDataset("csv has no header".into()))?;
    let column_names: Vec<&str> = header_line.split(',').map(|s| s.trim()).collect();
    if column_names.is_empty() {
        return Err(DataError::EmptyDataset("empty header".into()));
    }

    // Recoge filas primero para inferir tipos.
    let mut raw_rows: Vec<Vec<String>> = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cells: Vec<String> = line.split(',').map(|s| s.trim().to_string()).collect();
        raw_rows.push(cells);
    }

    // Infiere tipos por columna.
    let mut column_types: Vec<ColumnType> = vec![ColumnType::Unknown; column_names.len()];
    for row in &raw_rows {
        for (i, cell) in row.iter().enumerate() {
            if i >= column_types.len() {
                continue;
            }
            let inferred = ColumnType::infer(cell);
            column_types[i] = column_types[i].combine(inferred);
        }
    }

    // Construye columnas.
    let columns: Vec<DataColumn> = column_names
        .iter()
        .enumerate()
        .map(|(i, name)| DataColumn::new(*name, column_types[i]))
        .collect();

    // Construye tabla.
    let mut table = DataTable::new(columns);
    for row in raw_rows {
        let mut values: Vec<DataValue> = Vec::with_capacity(column_types.len());
        for (i, ty) in column_types.iter().enumerate() {
            let s = row.get(i).map(|s| s.as_str()).unwrap_or("");
            values.push(DataValue::parse(s, *ty));
        }
        table.push_row(values)?;
    }

    Ok(table)
}

/// Convierte un `serde_json::Value::Array` de objetos a `DataTable`.
fn json_array_to_table(value: &serde_json::Value) -> DataResult<DataTable> {
    let arr = match value {
        serde_json::Value::Array(a) => a,
        _ => {
            return Err(DataError::ParseError(
                "expected top-level JSON array".into(),
            ))
        }
    };

    if arr.is_empty() {
        return Err(DataError::EmptyDataset("empty JSON array".into()));
    }

    // Recoge todas las claves (unión).
    let mut keys: Vec<String> = Vec::new();
    for item in arr {
        if let serde_json::Value::Object(map) = item {
            for k in map.keys() {
                if !keys.contains(k) {
                    keys.push(k.clone());
                }
            }
        }
    }

    if keys.is_empty() {
        return Err(DataError::ParseError(
            "no object keys found in array".into(),
        ));
    }

    // Infiere tipos por columna.
    let mut column_types: Vec<ColumnType> = vec![ColumnType::Unknown; keys.len()];
    for item in arr {
        if let serde_json::Value::Object(map) = item {
            for (i, key) in keys.iter().enumerate() {
                if let Some(v) = map.get(key) {
                    let ty = json_value_type(v);
                    column_types[i] = column_types[i].combine(ty);
                }
            }
        }
    }

    let columns: Vec<DataColumn> = keys
        .iter()
        .enumerate()
        .map(|(i, name)| DataColumn::new(name, column_types[i]))
        .collect();

    let mut table = DataTable::new(columns);
    for item in arr {
        if let serde_json::Value::Object(map) = item {
            let mut values: Vec<DataValue> = Vec::with_capacity(keys.len());
            for key in &keys {
                let v = map.get(key).cloned().unwrap_or(serde_json::Value::Null);
                values.push(json_to_data_value(&v));
            }
            table.push_row(values)?;
        }
    }

    Ok(table)
}

/// Determina el `ColumnType` de un `serde_json::Value`.
fn json_value_type(v: &serde_json::Value) -> ColumnType {
    match v {
        serde_json::Value::Null => ColumnType::Unknown,
        serde_json::Value::Bool(_) => ColumnType::Bool,
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                ColumnType::Int
            } else {
                ColumnType::Float
            }
        }
        serde_json::Value::String(_) => ColumnType::Text,
        _ => ColumnType::Text,
    }
}

/// Convierte un `serde_json::Value` a `DataValue`.
fn json_to_data_value(v: &serde_json::Value) -> DataValue {
    match v {
        serde_json::Value::Null => DataValue::Null,
        serde_json::Value::Bool(b) => DataValue::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                DataValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                DataValue::Float(f)
            } else {
                DataValue::Null
            }
        }
        serde_json::Value::String(s) => DataValue::Text(s.clone()),
        other => DataValue::Text(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn format_display_names() {
        assert_eq!(DataFormat::Csv.display_name(), "csv");
        assert_eq!(DataFormat::Jsonl.display_name(), "jsonl");
        assert_eq!(DataFormat::Unknown.display_name(), "unknown");
    }

    #[test]
    fn format_is_supported() {
        assert!(DataFormat::Csv.is_supported());
        assert!(DataFormat::Json.is_supported());
        assert!(DataFormat::Jsonl.is_supported());
        assert!(!DataFormat::Parquet.is_supported());
        assert!(!DataFormat::Sqlite.is_supported());
    }

    #[test]
    fn format_from_path() {
        assert_eq!(
            DataFormat::from_path(Path::new("data.csv")).unwrap(),
            DataFormat::Csv
        );
        assert_eq!(
            DataFormat::from_path(Path::new("data.json")).unwrap(),
            DataFormat::Json
        );
        assert_eq!(
            DataFormat::from_path(Path::new("data.jsonl")).unwrap(),
            DataFormat::Jsonl
        );
        assert_eq!(
            DataFormat::from_path(Path::new("data.parquet")).unwrap(),
            DataFormat::Parquet
        );
    }

    #[test]
    fn load_missing_file_fails() {
        let loader = DataLoader::new();
        let err = loader.load("/definitely/not/a/file.csv").unwrap_err();
        assert!(matches!(err, DataError::NotFound(_)));
    }

    #[test]
    fn load_csv_simple() {
        let dir = TempDir::new().unwrap();
        let path = write(
            dir.path(),
            "data.csv",
            "name,age\nAlice,30\nBob,25\nCarol,40",
        );

        let loader = DataLoader::new();
        let table = loader.load(&path).unwrap();

        assert_eq!(table.row_count(), 3);
        assert_eq!(table.column_count(), 2);
        assert_eq!(table.column_names(), vec!["name", "age"]);
        assert_eq!(
            table.get(0, "name").unwrap(),
            &DataValue::Text("Alice".into())
        );
        assert_eq!(table.get(0, "age").unwrap(), &DataValue::Int(30));
        assert_eq!(table.get(2, "age").unwrap(), &DataValue::Int(40));
    }

    #[test]
    fn load_csv_infers_types() {
        let dir = TempDir::new().unwrap();
        let path = write(
            dir.path(),
            "data.csv",
            "name,age,score,active\nAlice,30,9.5,true\nBob,25,7.2,false",
        );

        let loader = DataLoader::new();
        let table = loader.load(&path).unwrap();

        assert_eq!(table.columns()[0].ty, ColumnType::Text);
        assert_eq!(table.columns()[1].ty, ColumnType::Int);
        assert_eq!(table.columns()[2].ty, ColumnType::Float);
        assert_eq!(table.columns()[3].ty, ColumnType::Bool);
    }

    #[test]
    fn load_csv_empty_fails() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "empty.csv", "");

        let loader = DataLoader::new();
        let err = loader.load(&path).unwrap_err();
        assert!(matches!(err, DataError::EmptyDataset(_)));
    }

    #[test]
    fn load_csv_only_header() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "header.csv", "name,age");

        let loader = DataLoader::new();
        let table = loader.load(&path).unwrap();
        assert_eq!(table.row_count(), 0);
        assert_eq!(table.column_count(), 2);
    }

    #[test]
    fn load_json_array() {
        let dir = TempDir::new().unwrap();
        let path = write(
            dir.path(),
            "data.json",
            r#"[
                {"name": "Alice", "age": 30},
                {"name": "Bob", "age": 25}
            ]"#,
        );

        let loader = DataLoader::new();
        let table = loader.load(&path).unwrap();

        assert_eq!(table.row_count(), 2);
        assert_eq!(table.column_count(), 2);
        assert_eq!(
            table.get(0, "name").unwrap(),
            &DataValue::Text("Alice".into())
        );
    }

    #[test]
    fn load_json_union_of_keys() {
        let dir = TempDir::new().unwrap();
        let path = write(
            dir.path(),
            "data.json",
            r#"[
                {"a": 1},
                {"b": 2}
            ]"#,
        );

        let loader = DataLoader::new();
        let table = loader.load(&path).unwrap();

        // Debe tener columnas a y b.
        let names = table.column_names();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        // El valor ausente es Null.
        assert_eq!(table.get(0, "b").unwrap(), &DataValue::Null);
    }

    #[test]
    fn load_json_not_array_fails() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "obj.json", r#"{"key": "value"}"#);

        let loader = DataLoader::new();
        let err = loader.load(&path).unwrap_err();
        assert!(matches!(err, DataError::ParseError(_)));
    }

    #[test]
    fn load_json_empty_array_fails() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "empty.json", "[]");

        let loader = DataLoader::new();
        let err = loader.load(&path).unwrap_err();
        assert!(matches!(err, DataError::EmptyDataset(_)));
    }

    #[test]
    fn load_jsonl() {
        let dir = TempDir::new().unwrap();
        let path = write(
            dir.path(),
            "data.jsonl",
            r#"{"name": "Alice", "age": 30}
{"name": "Bob", "age": 25}
{"name": "Carol", "age": 40}"#,
        );

        let loader = DataLoader::new();
        let table = loader.load(&path).unwrap();

        assert_eq!(table.row_count(), 3);
        assert_eq!(
            table.get(0, "name").unwrap(),
            &DataValue::Text("Alice".into())
        );
        assert_eq!(table.get(2, "age").unwrap(), &DataValue::Int(40));
    }

    #[test]
    fn load_jsonl_skips_empty_lines() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "data.jsonl", "{\"a\": 1}\n\n{\"a\": 2}\n");

        let loader = DataLoader::new();
        let table = loader.load(&path).unwrap();
        assert_eq!(table.row_count(), 2);
    }

    #[test]
    fn load_unsupported_fails() {
        let dir = TempDir::new().unwrap();
        let path = write(dir.path(), "data.parquet", "fake");

        let loader = DataLoader::new();
        let err = loader.load(&path).unwrap_err();
        assert!(matches!(err, DataError::UnsupportedFormat(_)));
    }

    #[test]
    fn full_flow_csv_load_and_query() {
        let dir = TempDir::new().unwrap();
        let path = write(
            dir.path(),
            "people.csv",
            "name,age,city\nAlice,30,Madrid\nBob,25,Barcelona\nCarol,40,Madrid",
        );

        let loader = DataLoader::new();
        let table = loader.load(&path).unwrap();

        let q = crate::query::DataQuery::new()
            .where_eq("city", DataValue::Text("Madrid".into()))
            .order_by("age", true);

        let result = q.execute(&table).unwrap();
        assert_eq!(result.row_count(), 2);
        assert_eq!(
            result.get(0, "name").unwrap(),
            &DataValue::Text("Alice".into())
        );
        assert_eq!(
            result.get(1, "name").unwrap(),
            &DataValue::Text("Carol".into())
        );
    }
}
