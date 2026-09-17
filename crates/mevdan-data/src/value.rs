//! Valores tipados y columnas.

use serde::{Deserialize, Serialize};

/// Tipo de columna.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    /// Número entero.
    Int,
    /// Número flotante.
    Float,
    /// Texto.
    Text,
    /// Booleano.
    Bool,
    /// Tipo aún no determinado (columna vacía).
    Unknown,
}

impl ColumnType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ColumnType::Int => "int",
            ColumnType::Float => "float",
            ColumnType::Text => "text",
            ColumnType::Bool => "bool",
            ColumnType::Unknown => "unknown",
        }
    }

    /// Infiere el tipo a partir de un valor textual.
    pub fn infer(s: &str) -> ColumnType {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return ColumnType::Unknown;
        }
        if trimmed.eq_ignore_ascii_case("true") || trimmed.eq_ignore_ascii_case("false") {
            return ColumnType::Bool;
        }
        if trimmed.parse::<i64>().is_ok() {
            return ColumnType::Int;
        }
        if trimmed.parse::<f64>().is_ok() {
            return ColumnType::Float;
        }
        ColumnType::Text
    }

    /// Combina dos tipos (por ejemplo, al procesar varias filas).
    ///
    /// Reglas:
    /// - `Unknown` + X → X.
    /// - `Int` + `Float` → `Float`.
    /// - Todo lo demás → `Text` si son distintos.
    pub fn combine(self, other: ColumnType) -> ColumnType {
        use ColumnType::*;
        match (self, other) {
            (Unknown, x) | (x, Unknown) => x,
            (Int, Int) => Int,
            (Float, Float) => Float,
            (Int, Float) | (Float, Int) => Float,
            (Text, Text) => Text,
            (Bool, Bool) => Bool,
            _ => Text,
        }
    }
}

/// Un valor tipado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum DataValue {
    Null,
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
}

impl DataValue {
    pub fn is_null(&self) -> bool {
        matches!(self, DataValue::Null)
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            DataValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            DataValue::Float(f) => Some(*f),
            DataValue::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            DataValue::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DataValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Interpreta el valor como texto (para mostrar).
    pub fn to_display_string(&self) -> String {
        match self {
            DataValue::Null => "".to_string(),
            DataValue::Int(i) => i.to_string(),
            DataValue::Float(f) => f.to_string(),
            DataValue::Text(s) => s.clone(),
            DataValue::Bool(b) => b.to_string(),
        }
    }

    /// Parsea un valor desde texto, dado un tipo esperado.
    pub fn parse(s: &str, ty: ColumnType) -> DataValue {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return DataValue::Null;
        }
        match ty {
            ColumnType::Int => trimmed
                .parse::<i64>()
                .map(DataValue::Int)
                .unwrap_or_else(|_| DataValue::Text(s.to_string())),
            ColumnType::Float => trimmed
                .parse::<f64>()
                .map(DataValue::Float)
                .unwrap_or_else(|_| DataValue::Text(s.to_string())),
            ColumnType::Bool => {
                if trimmed.eq_ignore_ascii_case("true") {
                    DataValue::Bool(true)
                } else if trimmed.eq_ignore_ascii_case("false") {
                    DataValue::Bool(false)
                } else {
                    DataValue::Text(s.to_string())
                }
            }
            ColumnType::Text => DataValue::Text(s.to_string()),
            ColumnType::Unknown => infer_value(s),
        }
    }
}

/// Infiere el tipo y valor desde texto.
fn infer_value(s: &str) -> DataValue {
    let ty = ColumnType::infer(s);
    DataValue::parse(s, ty)
}

/// Definición de columna.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataColumn {
    pub name: String,
    pub ty: ColumnType,
}

impl DataColumn {
    pub fn new(name: impl Into<String>, ty: ColumnType) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_type_display_names() {
        assert_eq!(ColumnType::Int.display_name(), "int");
        assert_eq!(ColumnType::Float.display_name(), "float");
        assert_eq!(ColumnType::Text.display_name(), "text");
        assert_eq!(ColumnType::Bool.display_name(), "bool");
        assert_eq!(ColumnType::Unknown.display_name(), "unknown");
    }

    #[test]
    fn infer_int() {
        assert_eq!(ColumnType::infer("42"), ColumnType::Int);
        assert_eq!(ColumnType::infer("-7"), ColumnType::Int);
    }

    #[test]
    fn infer_float() {
        assert_eq!(ColumnType::infer("3.14"), ColumnType::Float);
        assert_eq!(ColumnType::infer("-0.5"), ColumnType::Float);
    }

    #[test]
    fn infer_bool() {
        assert_eq!(ColumnType::infer("true"), ColumnType::Bool);
        assert_eq!(ColumnType::infer("FALSE"), ColumnType::Bool);
    }

    #[test]
    fn infer_text() {
        assert_eq!(ColumnType::infer("hello"), ColumnType::Text);
    }

    #[test]
    fn infer_empty() {
        assert_eq!(ColumnType::infer(""), ColumnType::Unknown);
        assert_eq!(ColumnType::infer("  "), ColumnType::Unknown);
    }

    #[test]
    fn combine_unknown() {
        assert_eq!(
            ColumnType::Unknown.combine(ColumnType::Int),
            ColumnType::Int
        );
        assert_eq!(
            ColumnType::Int.combine(ColumnType::Unknown),
            ColumnType::Int
        );
    }

    #[test]
    fn combine_int_float() {
        assert_eq!(
            ColumnType::Int.combine(ColumnType::Float),
            ColumnType::Float
        );
        assert_eq!(
            ColumnType::Float.combine(ColumnType::Int),
            ColumnType::Float
        );
    }

    #[test]
    fn combine_same() {
        assert_eq!(ColumnType::Int.combine(ColumnType::Int), ColumnType::Int);
        assert_eq!(ColumnType::Text.combine(ColumnType::Text), ColumnType::Text);
    }

    #[test]
    fn combine_mixed_becomes_text() {
        assert_eq!(ColumnType::Int.combine(ColumnType::Text), ColumnType::Text);
        assert_eq!(ColumnType::Bool.combine(ColumnType::Int), ColumnType::Text);
    }

    #[test]
    fn value_is_null() {
        assert!(DataValue::Null.is_null());
        assert!(!DataValue::Int(0).is_null());
    }

    #[test]
    fn value_accessors() {
        assert_eq!(DataValue::Int(42).as_int(), Some(42));
        assert_eq!(DataValue::Float(2.71).as_float(), Some(2.71));
        assert_eq!(DataValue::Int(5).as_float(), Some(5.0));
        assert_eq!(DataValue::Text("hi".into()).as_text(), Some("hi"));
        assert_eq!(DataValue::Bool(true).as_bool(), Some(true));

        assert_eq!(DataValue::Text("x".into()).as_int(), None);
        assert_eq!(DataValue::Int(5).as_text(), None);
    }

    #[test]
    fn value_to_display_string() {
        assert_eq!(DataValue::Null.to_display_string(), "");
        assert_eq!(DataValue::Int(42).to_display_string(), "42");
        assert_eq!(DataValue::Text("hi".into()).to_display_string(), "hi");
        assert_eq!(DataValue::Bool(true).to_display_string(), "true");
    }

    #[test]
    fn value_parse_int() {
        assert_eq!(DataValue::parse("42", ColumnType::Int), DataValue::Int(42));
        assert_eq!(DataValue::parse("", ColumnType::Int), DataValue::Null);
    }

    #[test]
    fn value_parse_float() {
        assert_eq!(
            DataValue::parse("2.71", ColumnType::Float),
            DataValue::Float(2.71)
        );
    }

    #[test]
    fn value_parse_bool() {
        assert_eq!(
            DataValue::parse("true", ColumnType::Bool),
            DataValue::Bool(true)
        );
        assert_eq!(
            DataValue::parse("FALSE", ColumnType::Bool),
            DataValue::Bool(false)
        );
    }

    #[test]
    fn value_parse_text() {
        assert_eq!(
            DataValue::parse("hello", ColumnType::Text),
            DataValue::Text("hello".into())
        );
    }

    #[test]
    fn value_parse_unknown_infers() {
        assert_eq!(
            DataValue::parse("42", ColumnType::Unknown),
            DataValue::Int(42)
        );
        assert_eq!(
            DataValue::parse("hi", ColumnType::Unknown),
            DataValue::Text("hi".into())
        );
    }

    #[test]
    fn data_column_new() {
        let c = DataColumn::new("age", ColumnType::Int);
        assert_eq!(c.name, "age");
        assert_eq!(c.ty, ColumnType::Int);
    }

    #[test]
    fn value_serializes() {
        let v = DataValue::Int(42);
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["type"], "int");
        assert_eq!(json["value"], 42);

        let back: DataValue = serde_json::from_value(json).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn null_serializes() {
        let v = DataValue::Null;
        let json = serde_json::to_value(&v).unwrap();
        assert_eq!(json["type"], "null");
    }
}
