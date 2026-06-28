#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod db;
mod sync;

use chrono_tz::Tz;
use commands::chart::get_portfolio_history;
use commands::get_rate;
use commands::holdings::get_holdings;
use commands::sync::{
    force_update_all_fx, force_update_all_prices, force_update_one_currency_fx,
    force_update_one_listing_prices, sync, sync_fx, sync_prices,
};
use commands::trade::{buy, get_listings, import_buy_csv};
use db::init_db;
use reqwest;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta_typescript::Typescript;
use std::str::FromStr;
use sync::yahoo::Error as YahooError;
use tauri::{Manager, async_runtime::block_on};
use tauri_specta::{Builder, collect_commands};
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

    #[error("Timeout error: {0}")]
    Timeout(String),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound,
            _ => AppError::Database(err.to_string()),
        }
    }
}

#[derive(Clone)]
pub struct HttpClient {
    pub client: reqwest::Client,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = Builder::new().commands(collect_commands![
        get_holdings,
        get_portfolio_history,
        sync,
        sync_fx,
        sync_prices,
        force_update_all_prices,
        force_update_one_listing_prices,
        force_update_all_fx,
        force_update_one_currency_fx,
        get_listings,
        buy,
        import_buy_csv,
        get_rate
    ]);

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

            // store shared reqwest client
            let http = HttpClient {
                client: reqwest::Client::new(),
            };
            app.manage(http);

            // tauri specta: required to use events
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Error while running tauri application");
}

pub fn yahoo_suffix(mic: &str) -> Result<&'static str, YahooError> {
    match mic {
        // United States (no suffix)
        "XNAS" | "XNYS" | "NYSE" | "ARCX" | "XASE" | "XPHL" | "XBOS" | "IEXG" => Ok(""),

        // Canada
        "XTSE" => Ok(".TO"),
        "XTSX" => Ok(".V"),

        // United Kingdom
        "XLON" => Ok(".L"),

        // Euronext
        "XAMS" => Ok(".AS"),
        "XPAR" => Ok(".PA"),
        "XBRU" => Ok(".BR"),
        "XLIS" => Ok(".LS"),

        // Germany
        "XETR" => Ok(".DE"),
        "XFRA" => Ok(".F"),
        "XSTU" => Ok(".SG"),

        // Switzerland
        "XSWX" => Ok(".SW"),

        // Italy
        "XMIL" => Ok(".MI"),

        // Nordics
        "XSTO" => Ok(".ST"),
        "XCSE" => Ok(".CO"),
        "XHEL" => Ok(".HE"),
        "XOSL" => Ok(".OL"),

        // Japan
        "XTKS" => Ok(".T"),

        // Hong Kong
        "XHKG" => Ok(".HK"),

        // Australia
        "XASX" => Ok(".AX"),

        other => Err(crate::sync::yahoo::Error::Parse(format!(
            "Unknown exchange MIC '{other}': add it to yahoo_suffix() before syncing"
        ))),
    }
}

pub fn mic_timezone(mic: &str) -> Result<Tz, YahooError> {
    use chrono_tz::{America, Asia, Australia, Europe};

    match mic {
        // United States (mostly Eastern Time)
        "XNAS" | "XNYS" | "NYSE" | "ARCX" | "XASE" | "XPHL" | "XBOS" | "IEXG" => {
            Ok(America::New_York)
        }
        "XCHI" | "XCBO" => Ok(America::Chicago),

        // Canada
        "XTSE" | "XTSX" | "XMOD" => Ok(America::Toronto),

        // United Kingdom
        "XLON" | "XOFF" => Ok(Europe::London),

        // Euronext
        "XAMS" => Ok(Europe::Amsterdam),
        "XPAR" => Ok(Europe::Paris),
        "XBRU" => Ok(Europe::Brussels),
        "XLIS" => Ok(Europe::Lisbon),
        "XDUB" => Ok(Europe::Dublin),

        // Germany
        "XETR" | "XFRA" | "XSTU" => Ok(Europe::Berlin),

        // Switzerland
        "XSWX" => Ok(Europe::Zurich),

        // Italy
        "XMIL" => Ok(Europe::Rome),

        // Nordics
        "XSTO" => Ok(Europe::Stockholm),
        "XCSE" => Ok(Europe::Copenhagen),
        "XHEL" => Ok(Europe::Helsinki),
        "XOSL" => Ok(Europe::Oslo),

        // Japan
        "XTKS" | "XOSJ" => Ok(Asia::Tokyo),

        // Hong Kong
        "XHKG" => Ok(Asia::Hong_Kong),

        // China
        "XSHG" | "XSHE" => Ok(Asia::Shanghai),

        // Australia
        "XASX" => Ok(Australia::Sydney),

        // India
        "XBOM" | "XNSE" => Ok(Asia::Kolkata),

        other => Err(YahooError::Parse(format!(
            "Unknown exchange MIC '{other}': add it to mic_timezone() before syncing"
        ))),
    }
}

// EEA domicile codes used to determine the 0.12% accumulating rate.
// EEA = EU member states + Norway, Iceland, Liechtenstein.
const EEA_DOMICILES: [&str; 30] = [
    "AT", "BE", "BG", "HR", "CY", "CZ", "DK", "EE", "FI", "FR", "DE", "GR", "HU", "IS", "IE", "IT",
    "LV", "LI", "LT", "LU", "MT", "NL", "NO", "PL", "PT", "RO", "SK", "SI", "ES", "SE",
];
