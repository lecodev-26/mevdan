//! `DataTable` — tabla en memoria.

use crate::{
    error::{DataError, DataResult},
    value::{ColumnType, DataColumn, DataValue},
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

/// Tabla de datos en memoria.
#[derive(Debug, Clone, Serialize)]
pub struct DataTable {
    columns: Vec<DataColumn>,
    rows: Vec<Vec<DataValue>>,
    /// Índice nombre → posición para acceso rápido.
    /// Se reconstruye automáticamente al deserializar.
    #[serde(skip)]
    column_index: BTreeMap<String, usize>,
}

/// Helper para deserializar sin `column_index`.
#[derive(Deserialize)]
struct DataTableRepr {
    columns: Vec<DataColumn>,
    rows: Vec<Vec<DataValue>>,
}

impl<'de> Deserialize<'de> for DataTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let repr = DataTableRepr::deserialize(deserializer)?;
        let column_index = repr
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| (c.name.clone(), i))
            .collect();
        Ok(DataTable {
            columns: repr.columns,
            rows: repr.rows,
            column_index,
        })
    }
}

impl DataTable {
    /// Crea una tabla vacía con las columnas dadas.
    pub fn new(columns: Vec<DataColumn>) -> Self {
        let column_index = columns
            .iter()
            .enumerate()
            .map(|(i, c)| (c.name.clone(), i))
            .collect();
        Self {
            columns,
            rows: Vec::new(),
            column_index,
        }
    }

    /// Añade una fila. La longitud debe coincidir con las columnas.
    pub fn push_row(&mut self, row: Vec<DataValue>) -> DataResult<()> {
        if row.len() != self.columns.len() {
            return Err(DataError::ParseError(format!(
                "row has {} values, expected {}",
                row.len(),
                self.columns.len()
            )));
        }
        self.rows.push(row);
        Ok(())
    }

    /// Nombres de las columnas.
    pub fn column_names(&self) -> Vec<&str> {
        self.columns.iter().map(|c| c.name.as_str()).collect()
    }

    /// Columnas.
    pub fn columns(&self) -> &[DataColumn] {
        &self.columns
    }

    /// Número de columnas.
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// Número de filas.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// ¿Está vacía?
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Todas las filas.
    pub fn rows(&self) -> &[Vec<DataValue>] {
        &self.rows
    }

    /// Índice de una columna.
    pub fn column_index(&self, name: &str) -> DataResult<usize> {
        self.column_index
            .get(name)
            .copied()
            .ok_or_else(|| DataError::ColumnNotFound(name.to_string()))
    }

    /// Obtiene una fila por índice.
    pub fn row(&self, idx: usize) -> DataResult<&[DataValue]> {
        self.rows
            .get(idx)
            .map(|r| r.as_slice())
            .ok_or(DataError::RowOutOfBounds(idx))
    }

    /// Obtiene un valor por (fila, columna).
    pub fn get(&self, row_idx: usize, column_name: &str) -> DataResult<&DataValue> {
        let col_idx = self.column_index(column_name)?;
        let row = self.row(row_idx)?;
        row.get(col_idx)
            .ok_or_else(|| DataError::ColumnNotFound(column_name.to_string()))
    }

    /// Todas las filas como slice.
    pub fn all_rows(&self) -> &[Vec<DataValue>] {
        &self.rows
    }

    /// Resumen textual.
    pub fn summary(&self) -> String {
        let cols: Vec<String> = self
            .columns
            .iter()
            .map(|c| format!("{}:{}", c.name, c.ty.display_name()))
            .collect();
        format!(
            "DataTable [{} rows × {} cols] ({})",
            self.row_count(),
            self.column_count(),
            cols.join(", ")
        )
    }

    /// Añade una columna calculada a partir de un closure.
    pub fn add_computed_column<F>(
        &mut self,
        name: impl Into<String>,
        ty: ColumnType,
        f: F,
    ) -> DataResult<()>
    where
        F: Fn(&[DataValue]) -> DataValue,
    {
        let new_col = DataColumn::new(name.into(), ty);
        let idx = self.columns.len();

        for row in &mut self.rows {
            let value = f(row);
            row.push(value);
        }
        self.columns.push(new_col);
        self.column_index
            .insert(self.columns[idx].name.clone(), idx);
        Ok(())
    }

    /// Filtra filas por un predicado.
    pub fn filter<F>(&self, predicate: F) -> DataTable
    where
        F: Fn(&[DataValue]) -> bool,
    {
        let filtered: Vec<Vec<DataValue>> =
            self.rows.iter().filter(|r| predicate(r)).cloned().collect();
        let mut table = DataTable::new(self.columns.clone());
        table.rows = filtered;
        table
    }

    /// Selecciona columnas por nombre.
    pub fn select(&self, columns: &[&str]) -> DataResult<DataTable> {
        let indices: Vec<usize> = columns
            .iter()
            .map(|name| self.column_index(name))
            .collect::<DataResult<Vec<_>>>()?;

        let new_columns: Vec<DataColumn> =
            indices.iter().map(|&i| self.columns[i].clone()).collect();

        let new_rows: Vec<Vec<DataValue>> = self
            .rows
            .iter()
            .map(|row| indices.iter().map(|&i| row[i].clone()).collect())
            .collect();

        let mut table = DataTable::new(new_columns);
        table.rows = new_rows;
        Ok(table)
    }

    /// Devuelve las primeras `n` filas.
    pub fn head(&self, n: usize) -> DataTable {
        let mut table = DataTable::new(self.columns.clone());
        table.rows = self.rows.iter().take(n).cloned().collect();
        table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table() -> DataTable {
        let mut t = DataTable::new(vec![
            DataColumn::new("name", ColumnType::Text),
            DataColumn::new("age", ColumnType::Int),
            DataColumn::new("active", ColumnType::Bool),
        ]);
        t.push_row(vec![
            DataValue::Text("Alice".into()),
            DataValue::Int(30),
            DataValue::Bool(true),
        ])
        .unwrap();
        t.push_row(vec![
            DataValue::Text("Bob".into()),
            DataValue::Int(25),
            DataValue::Bool(false),
        ])
        .unwrap();
        t.push_row(vec![
            DataValue::Text("Carol".into()),
            DataValue::Int(40),
            DataValue::Bool(true),
        ])
        .unwrap();
        t
    }

    #[test]
    fn new_creates_empty_table() {
        let t = DataTable::new(vec![DataColumn::new("x", ColumnType::Int)]);
        assert_eq!(t.column_count(), 1);
        assert_eq!(t.row_count(), 0);
        assert!(t.is_empty());
    }

    #[test]
    fn push_row_works() {
        let mut t = DataTable::new(vec![DataColumn::new("x", ColumnType::Int)]);
        t.push_row(vec![DataValue::Int(1)]).unwrap();
        assert_eq!(t.row_count(), 1);
    }

    #[test]
    fn push_row_wrong_length_fails() {
        let mut t = DataTable::new(vec![DataColumn::new("x", ColumnType::Int)]);
        let err = t
            .push_row(vec![DataValue::Int(1), DataValue::Int(2)])
            .unwrap_err();
        assert!(matches!(err, DataError::ParseError(_)));
    }

    #[test]
    fn column_names() {
        let t = sample_table();
        assert_eq!(t.column_names(), vec!["name", "age", "active"]);
    }

    #[test]
    fn column_index_works() {
        let t = sample_table();
        assert_eq!(t.column_index("name").unwrap(), 0);
        assert_eq!(t.column_index("age").unwrap(), 1);
        assert!(t.column_index("nope").is_err());
    }

    #[test]
    fn row_by_index() {
        let t = sample_table();
        let row = t.row(0).unwrap();
        assert_eq!(row[0], DataValue::Text("Alice".into()));
    }

    #[test]
    fn row_out_of_bounds() {
        let t = sample_table();
        assert!(t.row(10).is_err());
    }

    #[test]
    fn get_value_by_name() {
        let t = sample_table();
        assert_eq!(t.get(0, "name").unwrap(), &DataValue::Text("Alice".into()));
        assert_eq!(t.get(1, "age").unwrap(), &DataValue::Int(25));
    }

    #[test]
    fn get_missing_column_fails() {
        let t = sample_table();
        let err = t.get(0, "nope").unwrap_err();
        assert!(matches!(err, DataError::ColumnNotFound(_)));
    }

    #[test]
    fn summary_works() {
        let t = sample_table();
        let s = t.summary();
        assert!(s.contains("3 rows"));
        assert!(s.contains("3 cols"));
        assert!(s.contains("name:text"));
        assert!(s.contains("age:int"));
        assert!(s.contains("active:bool"));
    }

    #[test]
    fn filter_works() {
        let t = sample_table();
        let adults = t.filter(|row| match &row[1] {
            DataValue::Int(age) => *age >= 30,
            _ => false,
        });
        assert_eq!(adults.row_count(), 2);
        assert_eq!(
            adults.get(0, "name").unwrap(),
            &DataValue::Text("Alice".into())
        );
        assert_eq!(
            adults.get(1, "name").unwrap(),
            &DataValue::Text("Carol".into())
        );
    }

    #[test]
    fn select_works() {
        let t = sample_table();
        let selected = t.select(&["name", "age"]).unwrap();
        assert_eq!(selected.column_count(), 2);
        assert_eq!(selected.column_names(), vec!["name", "age"]);
        assert_eq!(selected.row_count(), 3);
    }

    #[test]
    fn select_missing_column_fails() {
        let t = sample_table();
        assert!(t.select(&["nope"]).is_err());
    }

    #[test]
    fn head_limits_rows() {
        let t = sample_table();
        let head = t.head(2);
        assert_eq!(head.row_count(), 2);
    }

    #[test]
    fn head_more_than_available() {
        let t = sample_table();
        let head = t.head(100);
        assert_eq!(head.row_count(), 3);
    }

    #[test]
    fn add_computed_column_works() {
        let mut t = sample_table();
        t.add_computed_column("is_adult", ColumnType::Bool, |row| match &row[1] {
            DataValue::Int(age) => DataValue::Bool(*age >= 30),
            _ => DataValue::Bool(false),
        })
        .unwrap();

        assert_eq!(t.column_count(), 4);
        assert_eq!(t.column_index("is_adult").unwrap(), 3);
        assert_eq!(t.get(0, "is_adult").unwrap(), &DataValue::Bool(true));
        assert_eq!(t.get(1, "is_adult").unwrap(), &DataValue::Bool(false));
    }

    #[test]
    fn table_serializes() {
        let t = sample_table();
        let json = serde_json::to_string(&t).unwrap();
        let back: DataTable = serde_json::from_str(&json).unwrap();
        assert_eq!(back.row_count(), t.row_count());
        assert_eq!(back.column_count(), t.column_count());
    }

    #[test]
    fn table_after_deserialize_allows_column_lookup() {
        let t = sample_table();
        let json = serde_json::to_string(&t).unwrap();
        let back: DataTable = serde_json::from_str(&json).unwrap();

        // El índice se reconstruye tras deserializar.
        assert_eq!(back.column_index("name").unwrap(), 0);
    }
}
