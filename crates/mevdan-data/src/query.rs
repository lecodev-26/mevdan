//! `DataQuery` — filtros simples sobre una tabla.

use crate::{
    error::{DataError, DataResult},
    table::DataTable,
    value::DataValue,
};
use serde::{Deserialize, Serialize};

/// Operador de comparación.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompareOp {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    /// Compara con texto (contains).
    Contains,
}

impl CompareOp {
    pub fn display_name(&self) -> &'static str {
        match self {
            CompareOp::Eq => "eq",
            CompareOp::Neq => "neq",
            CompareOp::Lt => "lt",
            CompareOp::Lte => "lte",
            CompareOp::Gt => "gt",
            CompareOp::Gte => "gte",
            CompareOp::Contains => "contains",
        }
    }

    /// Aplica el operador entre dos valores.
    pub fn apply(&self, left: &DataValue, right: &DataValue) -> bool {
        match self {
            CompareOp::Contains => match (left, right) {
                (DataValue::Text(l), DataValue::Text(r)) => l.contains(r.as_str()),
                _ => false,
            },
            _ => {
                // Compara numéricamente si ambos son numéricos.
                if let (Some(l), Some(r)) = (left.as_float(), right.as_float()) {
                    return match self {
                        CompareOp::Eq => l == r,
                        CompareOp::Neq => l != r,
                        CompareOp::Lt => l < r,
                        CompareOp::Lte => l <= r,
                        CompareOp::Gt => l > r,
                        CompareOp::Gte => l >= r,
                        CompareOp::Contains => false,
                    };
                }
                // Compara como texto si ambos son texto.
                match (left, right) {
                    (DataValue::Text(l), DataValue::Text(r)) => match self {
                        CompareOp::Eq => l == r,
                        CompareOp::Neq => l != r,
                        CompareOp::Lt => l < r,
                        CompareOp::Lte => l <= r,
                        CompareOp::Gt => l > r,
                        CompareOp::Gte => l >= r,
                        CompareOp::Contains => l.contains(r.as_str()),
                    },
                    (DataValue::Bool(l), DataValue::Bool(r)) => match self {
                        CompareOp::Eq => l == r,
                        CompareOp::Neq => l != r,
                        _ => false,
                    },
                    _ => false,
                }
            }
        }
    }
}

/// Filtro de una query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub column: String,
    pub op: CompareOp,
    pub value: DataValue,
}

impl Filter {
    pub fn new(column: impl Into<String>, op: CompareOp, value: DataValue) -> Self {
        Self {
            column: column.into(),
            op,
            value,
        }
    }

    pub fn eq(column: impl Into<String>, value: DataValue) -> Self {
        Self::new(column, CompareOp::Eq, value)
    }

    pub fn gt(column: impl Into<String>, value: DataValue) -> Self {
        Self::new(column, CompareOp::Gt, value)
    }

    pub fn gte(column: impl Into<String>, value: DataValue) -> Self {
        Self::new(column, CompareOp::Gte, value)
    }

    pub fn lt(column: impl Into<String>, value: DataValue) -> Self {
        Self::new(column, CompareOp::Lt, value)
    }

    pub fn lte(column: impl Into<String>, value: DataValue) -> Self {
        Self::new(column, CompareOp::Lte, value)
    }

    pub fn contains(column: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(column, CompareOp::Contains, DataValue::Text(value.into()))
    }

    /// Evalúa el filtro sobre una fila.
    pub fn matches(&self, table: &DataTable, row_idx: usize) -> DataResult<bool> {
        let col_idx = table.column_index(&self.column)?;
        let row = table.row(row_idx)?;
        let left = row
            .get(col_idx)
            .ok_or_else(|| DataError::ColumnNotFound(self.column.clone()))?;
        Ok(self.op.apply(left, &self.value))
    }
}

/// Combinador lógico de filtros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogicalOp {
    And,
    Or,
}

/// Una query de datos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQuery {
    pub filters: Vec<Filter>,
    pub logical: LogicalOp,
    pub select: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub order_by: Option<OrderBy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBy {
    pub column: String,
    pub ascending: bool,
}

impl DataQuery {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
            logical: LogicalOp::And,
            select: None,
            limit: None,
            order_by: None,
        }
    }

    pub fn where_eq(mut self, column: impl Into<String>, value: DataValue) -> Self {
        self.filters.push(Filter::eq(column, value));
        self
    }

    pub fn where_gt(mut self, column: impl Into<String>, value: DataValue) -> Self {
        self.filters.push(Filter::gt(column, value));
        self
    }

    pub fn where_gte(mut self, column: impl Into<String>, value: DataValue) -> Self {
        self.filters.push(Filter::gte(column, value));
        self
    }

    pub fn where_lt(mut self, column: impl Into<String>, value: DataValue) -> Self {
        self.filters.push(Filter::lt(column, value));
        self
    }

    pub fn where_contains(mut self, column: impl Into<String>, value: impl Into<String>) -> Self {
        self.filters.push(Filter::contains(column, value));
        self
    }

    pub fn and(mut self) -> Self {
        self.logical = LogicalOp::And;
        self
    }

    pub fn or(mut self) -> Self {
        self.logical = LogicalOp::Or;
        self
    }

    pub fn select(mut self, columns: Vec<String>) -> Self {
        self.select = Some(columns);
        self
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }

    pub fn order_by(mut self, column: impl Into<String>, ascending: bool) -> Self {
        self.order_by = Some(OrderBy {
            column: column.into(),
            ascending,
        });
        self
    }

    /// Ejecuta la query sobre una tabla.
    pub fn execute(&self, table: &DataTable) -> DataResult<DataTable> {
        // 1. Filtrar.
        let mut matching_rows: Vec<usize> = Vec::new();
        for idx in 0..table.row_count() {
            if self.filters.is_empty() {
                matching_rows.push(idx);
                continue;
            }
            let matched = match self.logical {
                LogicalOp::And => self
                    .filters
                    .iter()
                    .all(|f| f.matches(table, idx).unwrap_or(false)),
                LogicalOp::Or => self
                    .filters
                    .iter()
                    .any(|f| f.matches(table, idx).unwrap_or(false)),
            };
            if matched {
                matching_rows.push(idx);
            }
        }

        // 2. Ordenar.
        if let Some(order) = &self.order_by {
            let col_idx = table.column_index(&order.column)?;
            matching_rows.sort_by(|&a, &b| {
                let va = &table.rows()[a][col_idx];
                let vb = &table.rows()[b][col_idx];
                let cmp = compare_values(va, vb);
                if order.ascending {
                    cmp
                } else {
                    cmp.reverse()
                }
            });
        }

        // 3. Aplicar límite.
        if let Some(n) = self.limit {
            matching_rows.truncate(n);
        }

        // 4. Construir tabla resultado.
        let rows: Vec<Vec<DataValue>> = matching_rows
            .iter()
            .map(|&i| table.rows()[i].clone())
            .collect();

        let mut result = DataTable::new(table.columns().to_vec());
        for row in rows {
            result.push_row(row)?;
        }

        // 5. Proyección (SELECT).
        if let Some(cols) = &self.select {
            let refs: Vec<&str> = cols.iter().map(|s| s.as_str()).collect();
            return result.select(&refs);
        }

        Ok(result)
    }
}

impl Default for DataQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Compara dos valores para ordenación.
fn compare_values(a: &DataValue, b: &DataValue) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (DataValue::Int(x), DataValue::Int(y)) => x.cmp(y),
        (DataValue::Float(x), DataValue::Float(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (DataValue::Int(x), DataValue::Float(y)) => {
            (*x as f64).partial_cmp(y).unwrap_or(Ordering::Equal)
        }
        (DataValue::Float(x), DataValue::Int(y)) => {
            x.partial_cmp(&(*y as f64)).unwrap_or(Ordering::Equal)
        }
        (DataValue::Text(x), DataValue::Text(y)) => x.cmp(y),
        (DataValue::Bool(x), DataValue::Bool(y)) => x.cmp(y),
        (DataValue::Null, DataValue::Null) => Ordering::Equal,
        (DataValue::Null, _) => Ordering::Less,
        (_, DataValue::Null) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{ColumnType, DataColumn};

    fn sample_table() -> DataTable {
        let mut t = DataTable::new(vec![
            DataColumn::new("name", ColumnType::Text),
            DataColumn::new("age", ColumnType::Int),
            DataColumn::new("city", ColumnType::Text),
        ]);
        t.push_row(vec![
            DataValue::Text("Alice".into()),
            DataValue::Int(30),
            DataValue::Text("Madrid".into()),
        ])
        .unwrap();
        t.push_row(vec![
            DataValue::Text("Bob".into()),
            DataValue::Int(25),
            DataValue::Text("Barcelona".into()),
        ])
        .unwrap();
        t.push_row(vec![
            DataValue::Text("Carol".into()),
            DataValue::Int(40),
            DataValue::Text("Madrid".into()),
        ])
        .unwrap();
        t
    }

    #[test]
    fn compare_op_display_names() {
        assert_eq!(CompareOp::Eq.display_name(), "eq");
        assert_eq!(CompareOp::Contains.display_name(), "contains");
    }

    #[test]
    fn compare_op_numeric() {
        let a = DataValue::Int(10);
        let b = DataValue::Int(20);
        assert!(CompareOp::Lt.apply(&a, &b));
        assert!(CompareOp::Gt.apply(&b, &a));
        assert!(CompareOp::Lte.apply(&a, &b));
        assert!(CompareOp::Lte.apply(&a, &a));
        assert!(CompareOp::Eq.apply(&a, &a));
        assert!(CompareOp::Neq.apply(&a, &b));
    }

    #[test]
    fn compare_op_text() {
        let a = DataValue::Text("apple".into());
        let b = DataValue::Text("banana".into());
        assert!(CompareOp::Lt.apply(&a, &b));
        assert!(CompareOp::Contains.apply(&b, &DataValue::Text("nan".into())));
        assert!(!CompareOp::Contains.apply(&a, &DataValue::Text("ban".into())));
    }

    #[test]
    fn compare_op_bool() {
        let t = DataValue::Bool(true);
        let f = DataValue::Bool(false);
        assert!(CompareOp::Eq.apply(&t, &t));
        assert!(CompareOp::Neq.apply(&t, &f));
    }

    #[test]
    fn filter_eq() {
        let t = sample_table();
        let f = Filter::eq("name", DataValue::Text("Alice".into()));
        assert!(f.matches(&t, 0).unwrap());
        assert!(!f.matches(&t, 1).unwrap());
    }

    #[test]
    fn filter_gt() {
        let t = sample_table();
        let f = Filter::gt("age", DataValue::Int(28));
        assert!(f.matches(&t, 0).unwrap()); // Alice 30
        assert!(!f.matches(&t, 1).unwrap()); // Bob 25
        assert!(f.matches(&t, 2).unwrap()); // Carol 40
    }

    #[test]
    fn filter_contains() {
        let t = sample_table();
        let f = Filter::contains("name", "li");
        assert!(f.matches(&t, 0).unwrap()); // Alice
        assert!(!f.matches(&t, 1).unwrap()); // Bob
    }

    #[test]
    fn query_empty_returns_all() {
        let t = sample_table();
        let q = DataQuery::new();
        let r = q.execute(&t).unwrap();
        assert_eq!(r.row_count(), 3);
    }

    #[test]
    fn query_where_eq() {
        let t = sample_table();
        let q = DataQuery::new().where_eq("city", DataValue::Text("Madrid".into()));
        let r = q.execute(&t).unwrap();
        assert_eq!(r.row_count(), 2);
    }

    #[test]
    fn query_where_gt() {
        let t = sample_table();
        let q = DataQuery::new().where_gt("age", DataValue::Int(28));
        let r = q.execute(&t).unwrap();
        assert_eq!(r.row_count(), 2);
    }

    #[test]
    fn query_multiple_and() {
        let t = sample_table();
        let q = DataQuery::new()
            .where_eq("city", DataValue::Text("Madrid".into()))
            .where_gt("age", DataValue::Int(35));
        let r = q.execute(&t).unwrap();
        // Solo Carol (40, Madrid).
        assert_eq!(r.row_count(), 1);
        assert_eq!(r.get(0, "name").unwrap(), &DataValue::Text("Carol".into()));
    }

    #[test]
    fn query_multiple_or() {
        let t = sample_table();
        let q = DataQuery::new()
            .where_eq("name", DataValue::Text("Alice".into()))
            .where_eq("name", DataValue::Text("Bob".into()))
            .or();
        let r = q.execute(&t).unwrap();
        assert_eq!(r.row_count(), 2);
    }

    #[test]
    fn query_limit() {
        let t = sample_table();
        let q = DataQuery::new().limit(2);
        let r = q.execute(&t).unwrap();
        assert_eq!(r.row_count(), 2);
    }

    #[test]
    fn query_order_by_asc() {
        let t = sample_table();
        let q = DataQuery::new().order_by("age", true);
        let r = q.execute(&t).unwrap();
        assert_eq!(r.get(0, "age").unwrap(), &DataValue::Int(25)); // Bob
        assert_eq!(r.get(1, "age").unwrap(), &DataValue::Int(30)); // Alice
        assert_eq!(r.get(2, "age").unwrap(), &DataValue::Int(40)); // Carol
    }

    #[test]
    fn query_order_by_desc() {
        let t = sample_table();
        let q = DataQuery::new().order_by("age", false);
        let r = q.execute(&t).unwrap();
        assert_eq!(r.get(0, "age").unwrap(), &DataValue::Int(40)); // Carol
        assert_eq!(r.get(2, "age").unwrap(), &DataValue::Int(25)); // Bob
    }

    #[test]
    fn query_select() {
        let t = sample_table();
        let q = DataQuery::new().select(vec!["name".into(), "age".into()]);
        let r = q.execute(&t).unwrap();
        assert_eq!(r.column_count(), 2);
        assert_eq!(r.column_names(), vec!["name", "age"]);
    }

    #[test]
    fn query_combined() {
        let t = sample_table();
        let q = DataQuery::new()
            .where_gt("age", DataValue::Int(20))
            .order_by("age", false)
            .limit(2)
            .select(vec!["name".into(), "age".into()]);

        let r = q.execute(&t).unwrap();
        assert_eq!(r.row_count(), 2);
        assert_eq!(r.column_count(), 2);
        assert_eq!(r.get(0, "age").unwrap(), &DataValue::Int(40)); // Carol
        assert_eq!(r.get(1, "age").unwrap(), &DataValue::Int(30)); // Alice
    }

    #[test]
    fn query_serializes() {
        let q = DataQuery::new()
            .where_gt("age", DataValue::Int(20))
            .limit(5);
        let json = serde_json::to_string(&q).unwrap();
        let back: DataQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(back.filters.len(), 1);
        assert_eq!(back.limit, Some(5));
    }
}
