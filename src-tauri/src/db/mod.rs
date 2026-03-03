use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::{fs, path::PathBuf};
use tauri::{App, Manager};

pub mod types;

pub struct Db {
    pub pool: SqlitePool,
}

pub async fn init_db(app: &App) -> Db {
    let db_path = if cfg!(dev) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("No parent of dir with Cargo.toml")
            .join("brood-dev.db")
    } else {
        let app_data = app
            .path()
            .app_data_dir()
            .expect("Failed to get app data dir");
        if !app_data.exists() {
            fs::create_dir_all(&app_data).expect("Failed to create app data directory");
        }
        app_data.join("brood.db")
    };

    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePool::connect_with(options)
        .await
        .expect("Failed to connect to sqlitepool");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    Db { pool }
}
