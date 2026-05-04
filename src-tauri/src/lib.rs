#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod db;

use commands::holdings::get_holdings;
use db::init_db;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta_typescript::Typescript;
use std::str::FromStr;
use tauri::{async_runtime::block_on, Manager};
use tauri_specta::{collect_commands, Builder};
use thiserror::Error;

pub fn parse_decimal(s: &str, ctx: &str) -> Result<Decimal, AppError> {
    Decimal::from_str(s).map_err(|_| AppError::Database(format!("Malformed {ctx}: {s}")))
}

#[derive(Debug, Error, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "data")]
pub enum AppError {
    #[error("Data not found")]
    NotFound,

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal server error")]
    Internal,
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound,
            _ => AppError::Database(err.to_string()),
        }
    }
}

#[derive(sqlx::FromRow, Serialize, Deserialize, specta::Type)]
pub struct Count {
    pub id: i64,
    pub value: i64,
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
    let builder = Builder::new().commands(collect_commands![greet, count, get_holdings]);

    #[cfg(debug_assertions)]
    builder
        .export(
            Typescript::default()
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
