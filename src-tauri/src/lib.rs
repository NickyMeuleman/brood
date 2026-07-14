#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod db;
mod sync;

use chrono_tz::{America, Asia, Australia, Europe, Tz};
use commands::chart::get_portfolio_history;
use commands::holdings::get_holdings;
use commands::sync::{
    force_update_all_fx, force_update_all_prices, force_update_one_currency_fx,
    force_update_one_listing_prices, sync, sync_fx, sync_prices,
};
use commands::trade::{broker_fee_hint, buy, get_listings, import_buy_csv};
use commands::{get_mics, get_price, get_rate};
use db::init_db;
use reqwest;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta_typescript::Typescript;
use std::str::FromStr;
use std::time::Duration;
use sync::yahoo::Error as YahooError;
use tauri::{Manager, async_runtime::block_on};
use tauri_specta::{Builder, collect_commands};
use thiserror::Error;

pub fn parse_decimal_internal(s: &str, ctx: &str) -> Result<Decimal, AppError> {
    Decimal::from_str(s)
        .map_err(|_| AppError::Internal(format!("Corrupt stored value ({ctx}): '{s}'")))
}

pub fn parse_decimal_external(s: &str, ctx: &str) -> Result<Decimal, AppError> {
    Decimal::from_str(s).map_err(|_| AppError::Validation(format!("Invalid {ctx}: '{s}'")))
}

#[derive(Debug, Error, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "data")]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    Validation(String),

    #[error("Missing market data: {0}")]
    MissingData(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => AppError::NotFound("Record not found".into()),
            sqlx::Error::Database(db_err)
                if db_err.message().contains("UNIQUE constraint failed") =>
            {
                AppError::Validation(
                    "Tried to insert a duplicate. This trade or order probably already exists"
                        .into(),
                )
            }
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
        get_rate,
        get_price,
        broker_fee_hint,
        get_mics
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
                client: reqwest::Client::builder()
                    .timeout(Duration::from_secs(10))
                    .build()
                    .expect("reqwest client build"),
            };
            app.manage(http);

            // tauri specta: required to use events
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Error while running tauri application");
}

pub struct ExchangeInfo {
    pub mic: &'static str,
    pub yahoo_suffix: &'static str,
    pub tz: Tz,
}

pub const SUPPORTED_EXCHANGES: &[ExchangeInfo] = &[
    // United States (no suffix)
    ExchangeInfo {
        mic: "XNAS",
        yahoo_suffix: "",
        tz: America::New_York,
    },
    ExchangeInfo {
        mic: "XNYS",
        yahoo_suffix: "",
        tz: America::New_York,
    },
    ExchangeInfo {
        mic: "NYSE",
        yahoo_suffix: "",
        tz: America::New_York,
    },
    ExchangeInfo {
        mic: "ARCX",
        yahoo_suffix: "",
        tz: America::New_York,
    },
    ExchangeInfo {
        mic: "XASE",
        yahoo_suffix: "",
        tz: America::New_York,
    },
    ExchangeInfo {
        mic: "XPHL",
        yahoo_suffix: "",
        tz: America::New_York,
    },
    ExchangeInfo {
        mic: "XBOS",
        yahoo_suffix: "",
        tz: America::New_York,
    },
    ExchangeInfo {
        mic: "IEXG",
        yahoo_suffix: "",
        tz: America::New_York,
    },
    // Canada
    ExchangeInfo {
        mic: "XTSE",
        yahoo_suffix: ".TO",
        tz: America::Toronto,
    },
    ExchangeInfo {
        mic: "XTSX",
        yahoo_suffix: ".V",
        tz: America::Toronto,
    },
    // United Kingdom
    ExchangeInfo {
        mic: "XLON",
        yahoo_suffix: ".L",
        tz: Europe::London,
    },
    // Euronext
    ExchangeInfo {
        mic: "XAMS",
        yahoo_suffix: ".AS",
        tz: Europe::Amsterdam,
    },
    ExchangeInfo {
        mic: "XPAR",
        yahoo_suffix: ".PA",
        tz: Europe::Paris,
    },
    ExchangeInfo {
        mic: "XBRU",
        yahoo_suffix: ".BR",
        tz: Europe::Brussels,
    },
    ExchangeInfo {
        mic: "XLIS",
        yahoo_suffix: ".LS",
        tz: Europe::Lisbon,
    },
    // Germany
    ExchangeInfo {
        mic: "XETR",
        yahoo_suffix: ".DE",
        tz: Europe::Berlin,
    },
    ExchangeInfo {
        mic: "XFRA",
        yahoo_suffix: ".F",
        tz: Europe::Berlin,
    },
    ExchangeInfo {
        mic: "XSTU",
        yahoo_suffix: ".SG",
        tz: Europe::Berlin,
    },
    // Switzerland
    ExchangeInfo {
        mic: "XSWX",
        yahoo_suffix: ".SW",
        tz: Europe::Zurich,
    },
    // Italy
    ExchangeInfo {
        mic: "XMIL",
        yahoo_suffix: ".MI",
        tz: Europe::Rome,
    },
    // Nordics
    ExchangeInfo {
        mic: "XSTO",
        yahoo_suffix: ".ST",
        tz: Europe::Stockholm,
    },
    ExchangeInfo {
        mic: "XCSE",
        yahoo_suffix: ".CO",
        tz: Europe::Copenhagen,
    },
    ExchangeInfo {
        mic: "XHEL",
        yahoo_suffix: ".HE",
        tz: Europe::Helsinki,
    },
    ExchangeInfo {
        mic: "XOSL",
        yahoo_suffix: ".OL",
        tz: Europe::Oslo,
    },
    // Japan
    ExchangeInfo {
        mic: "XTKS",
        yahoo_suffix: ".T",
        tz: Asia::Tokyo,
    },
    // Hong Kong
    ExchangeInfo {
        mic: "XHKG",
        yahoo_suffix: ".HK",
        tz: Asia::Hong_Kong,
    },
    // Australia
    ExchangeInfo {
        mic: "XASX",
        yahoo_suffix: ".AX",
        tz: Australia::Sydney,
    },
];

pub fn yahoo_suffix(mic: &str) -> Result<&'static str, YahooError> {
    SUPPORTED_EXCHANGES
        .iter()
        .find(|e| e.mic == mic)
        .map(|e| e.yahoo_suffix)
        .ok_or_else(|| {
            crate::sync::yahoo::Error::UnknownMic(format!(
                "Unknown exchange MIC '{mic}': add it to SUPPORTED_EXCHANGES before syncing"
            ))
        })
}

pub fn mic_timezone(mic: &str) -> Result<Tz, YahooError> {
    SUPPORTED_EXCHANGES
        .iter()
        .find(|e| e.mic == mic)
        .map(|e| e.tz)
        .ok_or_else(|| {
            crate::sync::yahoo::Error::UnknownMic(format!(
                "Unknown exchange MIC '{mic}': add it to SUPPORTED_EXCHANGES before syncing"
            ))
        })
}

// EEA domicile codes used to determine the 0.12% accumulating rate.
// EEA = EU member states + Norway, Iceland, Liechtenstein.
const EEA_DOMICILES: [&str; 30] = [
    "AT", "BE", "BG", "HR", "CY", "CZ", "DK", "EE", "FI", "FR", "DE", "GR", "HU", "IS", "IE", "IT",
    "LV", "LI", "LT", "LU", "MT", "NL", "NO", "PL", "PT", "RO", "SK", "SI", "ES", "SE",
];
