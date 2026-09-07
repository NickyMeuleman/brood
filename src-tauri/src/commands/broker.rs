use crate::AppError;
use crate::db::Db;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx;
use tauri::State;

#[derive(Debug, Clone, Serialize, Type)]
pub struct Broker {
    pub id: i64,
    pub name: String,
    pub broker_type: Option<BrokerType>,
}

#[tauri::command]
#[specta::specta]
pub async fn get_brokers(db: State<'_, Db>) -> Result<Vec<Broker>, AppError> {
    sqlx::query_as!(
        Broker,
        r#"
        SELECT
            id as "id!",
            name,
            broker_type as "broker_type: BrokerType"
        FROM
           broker
        ORDER BY
           name ASC
    "#
    )
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.into())
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Serialize, Deserialize, specta::Type)]
#[sqlx(type_name = "TEXT", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BrokerType {
    Bolero,
    Rebel,
    Medirect,
    Saxo,
}

#[derive(Debug, Deserialize, Type)]
pub struct CreateBrokerInput {
    pub name: String,
    pub broker_type: Option<BrokerType>,
}

#[tauri::command]
#[specta::specta]
pub async fn add_broker(db: State<'_, Db>, fields: CreateBrokerInput) -> Result<i64, AppError> {
    let id = sqlx::query_scalar!(
        r#"
            INSERT INTO broker (name, broker_type)
            VALUES (?1, ?2)
            RETURNING id
            "#,
        fields.name,
        fields.broker_type
    )
    .fetch_one(&db.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => {
            AppError::Validation(format!("A broker with name {} already exists", fields.name))
        }
        e => AppError::from(e),
    })?;

    Ok(id)
}
