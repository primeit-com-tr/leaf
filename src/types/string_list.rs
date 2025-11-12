use sea_orm::{
    ColIdx, DbErr, QueryResult, TryGetError, TryGetable,
    sea_query::{ArrayType, Nullable, Value, ValueType, ValueTypeErr},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StringList(pub Vec<String>);

impl StringList {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn push(&mut self, value: impl Into<String>) {
        self.0.push(value.into());
    }

    pub fn extend<I, S>(&mut self, iter: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.0.extend(iter.into_iter().map(|s| s.into()));
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn items(&self) -> &[String] {
        &self.0
    }

    pub fn into_inner(self) -> Vec<String> {
        self.0
    }
}

impl From<Vec<String>> for StringList {
    fn from(v: Vec<String>) -> Self {
        Self(v)
    }
}

impl Nullable for StringList {
    fn null() -> Value {
        Value::String(None)
    }
}

impl From<StringList> for Value {
    fn from(s: StringList) -> Self {
        Value::String(Some(serde_json::to_string(&s.0).unwrap()))
    }
}

impl TryGetable for StringList {
    fn try_get_by<I: ColIdx>(res: &QueryResult, idx: I) -> Result<Self, TryGetError> {
        // allow NULL -> empty vec
        let value: Option<String> = res.try_get_by(idx)?;
        match value {
            Some(s) => serde_json::from_str(&s)
                .map(StringList)
                .map_err(|e| TryGetError::DbErr(DbErr::Type(e.to_string()))),
            None => Ok(StringList(vec![])),
        }
    }
}

impl ValueType for StringList {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        match v {
            Value::String(Some(s)) => serde_json::from_str(&s).map_err(|_| ValueTypeErr),
            Value::Json(Some(j)) => serde_json::from_value(j).map_err(|_| ValueTypeErr),
            Value::String(None) | Value::Json(None) => Ok(StringList(vec![])),
            _ => Err(ValueTypeErr),
        }
    }

    fn type_name() -> String {
        "StringList".to_owned()
    }

    fn array_type() -> ArrayType {
        ArrayType::String
    }

    fn column_type() -> sea_orm::ColumnType {
        sea_orm::ColumnType::Text
    }
}
