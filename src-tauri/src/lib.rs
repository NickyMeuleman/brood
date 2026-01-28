#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod db;

use db::{init_db, Db};
use serde::{Deserialize, Serialize};
use specta_typescript::{BigIntExportBehavior, Typescript};
use tauri::{async_runtime::block_on, Manager, State};
use tauri_specta::{collect_commands, Builder};

#[derive(sqlx::FromRow, Serialize, Deserialize, specta::Type)]
pub struct Count {
    pub id: i64,
    pub value: i64,
}

#[tauri::command]
#[specta::specta]
async fn get_count(state: State<'_, Db>, id: i64) -> Result<Count, String> {
    sqlx::query_as!(Count, "SELECT id, value FROM counts WHERE id = ?", id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
async fn increment_count(state: State<'_, Db>, id: i64) -> Result<Count, String> {
    let mut tx = state.pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query!("UPDATE counts SET value = value + 1 WHERE id = ?;", id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let updated = sqlx::query_as!(Count, "SELECT id, value FROM counts WHERE id = ?", id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;

    Ok(updated)
}

#[tauri::command]
#[specta::specta]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
#[specta::specta]
fn count(to: u32) -> Vec<u32> {
    (0..=to).collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder =
        Builder::new().commands(collect_commands![greet, count, get_count, increment_count]);

    #[cfg(debug_assertions)]
    builder
        .export(
            // treat i64s as js numbers because sqlite integers are i64s, WARNING: possible data loss
            Typescript::default()
                .bigint(BigIntExportBehavior::Number)
                // make typescript ignore the generated bindings file
                .header("//@ts-nocheck"),
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            // init db and add it to the tauri managed state
            let db = block_on(init_db(&app));
            app.manage(db);

            // tauri specta: required to use events
            builder.mount_events(app);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Error while running tauri application");
}
