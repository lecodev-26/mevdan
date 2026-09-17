//! # mevdan-data
//!
//! Procesamiento de datos estructurados para MEVDAN.
//!
//! ## Concepto
//!
//! Un **dataset** es una tabla de datos tipados. `mevdan-data` carga
//! CSV, JSON y JSONL a una `DataTable` en memoria, y permite
//! consultarla con filtros simples.
//!
//! ## Formatos
//!
//! | Formato | Carga | Notas |
//! |---------|-------|-------|
//! | CSV | ✅ | Tipos inferidos por columna |
//! | JSON | ✅ | Array de objetos |
//! | JSONL | ✅ | Una línea, un objeto |
//! | Parquet | ❌ | Reconocido, no cargado (V5.5+) |
//! | SQLite | ❌ | Reconocido, no cargado (V5.5+) |
//!
//! ## Estado del proyecto
//!
//! - **V5.4** ✅ — `DataValue`, `DataTable`, `DataQuery`, `DataLoader`.
//!
//! ## Ejemplo
//!
//! ```no_run
//! use mevdan_data::{DataLoader, DataQuery, DataValue};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let loader = DataLoader::new();
//! let table = loader.load("/path/to/people.csv")?;
//!
//! println!("{}", table.summary());
//!
//! let query = DataQuery::new()
//!     .where_eq("city", DataValue::Text("Madrid".into()))
//!     .order_by("age", true)
//!     .limit(10);
//!
//! let result = query.execute(&table)?;
//! println!("Matched {} rows", result.row_count());
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod loader;
pub mod query;
pub mod table;
pub mod value;

// Re-exports de conveniencia.
pub use error::{DataError, DataResult};
pub use loader::{DataFormat, DataLoader};
pub use query::{CompareOp, DataQuery, Filter, LogicalOp, OrderBy};
pub use table::DataTable;
pub use value::{ColumnType, DataColumn, DataValue};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn full_flow_csv_to_query() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sales.csv");
        fs::write(
            &path,
            "product,price,quantity\nApple,1.5,100\nBanana,0.5,200\nCherry,3.0,50",
        )
        .unwrap();

        let loader = DataLoader::new();
        let table = loader.load(&path).unwrap();

        assert_eq!(table.row_count(), 3);

        // Filtra por precio > 1.
        let q = DataQuery::new().where_gt("price", DataValue::Float(1.0));
        let result = q.execute(&table).unwrap();
        assert_eq!(result.row_count(), 2);

        // Ordena por cantidad desc.
        let q2 = DataQuery::new().order_by("quantity", false).limit(1);
        let top = q2.execute(&table).unwrap();
        assert_eq!(
            top.get(0, "product").unwrap(),
            &DataValue::Text("Banana".into())
        );
    }

    #[test]
    fn full_flow_json_to_query() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("users.json");
        fs::write(
            &path,
            r#"[
                {"name": "Alice", "score": 95, "vip": true},
                {"name": "Bob", "score": 72, "vip": false},
                {"name": "Carol", "score": 88, "vip": true}
            ]"#,
        )
        .unwrap();

        let loader = DataLoader::new();
        let table = loader.load(&path).unwrap();

        // Filtra VIPs.
        let q = DataQuery::new().where_eq("vip", DataValue::Bool(true));
        let vips = q.execute(&table).unwrap();
        assert_eq!(vips.row_count(), 2);

        // Filtra VIP con score > 90.
        let q = DataQuery::new()
            .where_eq("vip", DataValue::Bool(true))
            .where_gt("score", DataValue::Int(90));
        let top_vips = q.execute(&table).unwrap();
        assert_eq!(top_vips.row_count(), 1);
        assert_eq!(
            top_vips.get(0, "name").unwrap(),
            &DataValue::Text("Alice".into())
        );
    }

    #[test]
    fn full_flow_jsonl_to_query() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("events.jsonl");
        fs::write(
            &path,
            "{\"type\": \"click\", \"count\": 5}\n{\"type\": \"view\", \"count\": 10}\n{\"type\": \"click\", \"count\": 3}",
        )
        .unwrap();

        let loader = DataLoader::new();
        let table = loader.load(&path).unwrap();

        let q = DataQuery::new()
            .where_eq("type", DataValue::Text("click".into()))
            .order_by("count", false);

        let clicks = q.execute(&table).unwrap();
        assert_eq!(clicks.row_count(), 2);
        assert_eq!(clicks.get(0, "count").unwrap(), &DataValue::Int(5));
    }
}
