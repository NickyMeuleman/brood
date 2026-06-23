use crate::db::types::InstrumentType;
use crate::db::Db;
use crate::{AppError, HttpClient};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

#[derive(Debug, Serialize, Type)]
pub struct ListingInfo {
    pub id: i64,
    pub ticker: String,
    pub exchange_mic: String,
    pub currency_code: String,
    pub instrument_name: String,
    pub isin: String,
    pub instrument_type: InstrumentType,
}

#[tauri::command]
#[specta::specta]
pub async fn get_listings(db: State<'_, Db>) -> Result<Vec<ListingInfo>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT
            li.id               AS "id!",
            li.ticker           AS "ticker!",
            li.exchange_mic     AS "exchange_mic!",
            li.currency_code    AS "currency_code!",
            i.name              AS "instrument_name!",
            i.isin              AS "isin!",
            i.instrument_type   AS "instrument_type!: InstrumentType"
        FROM listing li
        JOIN instrument i ON i.id = li.instrument_id
        WHERE li.delisted_at IS NULL
        ORDER BY li.ticker
        "#
    )
    .fetch_all(&db.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ListingInfo {
            id: r.id,
            ticker: r.ticker,
            exchange_mic: r.exchange_mic,
            currency_code: r.currency_code,
            instrument_name: r.instrument_name,
            isin: r.isin,
            instrument_type: r.instrument_type,
        })
        .collect())
}

#[derive(Debug, Deserialize, Type)]
pub struct CreateBuyTradeInput {
    pub listing_id: i64,
    pub quantity: String,
    // pub price_per_unit: String,
    pub executed_at: DateTime<Utc>,
    // pub broker_fee: Option<String>,
    // pub tob_fee: Option<String>,
}
#[tauri::command]
#[specta::specta]
pub async fn buy(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
    fields: CreateBuyTradeInput,
) -> Result<String, AppError> {
    dbg!(&fields);
    let executed_at = fields.executed_at.naive_utc();
    dbg!(executed_at);
    Ok("wat".to_string())
}
