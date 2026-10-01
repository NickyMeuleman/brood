// Decimal values are stored as TEXT in SQLite. sqlx cannot decode
// TEXT into rust_decimal::Decimal directly, so all decimal fields
// are String here. Parse to Decimal when arithmetic is needed.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, Deserialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InstrumentType {
    Etf,
    Fund,
    Stock,
    Bond,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, Deserialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Replication {
    Physical,
    Synthetic,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FxRateSource {
    Ecb,
    Nbb,
    Broker,
    Manual,
}
