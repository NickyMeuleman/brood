use crate::AppError;
use crate::db::Db;
use serde::Serialize;
use specta::Type;
use sqlx;
use tauri::State;

#[derive(Debug, Clone, Serialize, Type)]
pub struct Broker {
    pub id: i64,
    pub name: String,
}

#[tauri::command]
#[specta::specta]
pub async fn get_brokers(db: State<'_, Db>) -> Result<Vec<Broker>, AppError> {
    sqlx::query_as!(
        Broker,
        r#"
        SELECT
            id as "id!",
            name
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
