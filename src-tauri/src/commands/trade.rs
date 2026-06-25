use crate::db::types::InstrumentType;
use crate::db::Db;
use crate::{parse_decimal, AppError, HttpClient};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta::Type;
use sqlx::{Pool, Sqlite};
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
    pub unit_price: String,
    pub executed_at: DateTime<Utc>,
    pub broker_fee: Option<String>,
    pub tob_fee: Option<String>,
}

async fn buy_core(pool: &Pool<Sqlite>, fields: CreateBuyTradeInput) -> Result<(), AppError> {
    let quantity = parse_decimal(&fields.quantity, "quantity")?;
    if quantity <= Decimal::ZERO {
        return Err(AppError::Database("quantity must be positive".into()));
    }
    let quantity_str = quantity.to_string();
    let unit_price = parse_decimal(&fields.unit_price, "unit_price")?;
    if unit_price <= Decimal::ZERO {
        return Err(AppError::Database("unit_price must be positive".into()));
    }
    let unit_price_str = unit_price.to_string();

    let parse_optional_fee =
        |raw: Option<String>, ctx: &'static str| -> Result<Option<Decimal>, AppError> {
            match raw {
                None => Ok(None),
                Some(v) if v.is_empty() || v == "0" => Ok(None),
                Some(v) => {
                    let d = parse_decimal(&v, ctx)?;
                    if d < Decimal::ZERO {
                        return Err(AppError::Database(format!("{ctx} must be non-negative")));
                    }
                    Ok(Some(d))
                }
            }
        };
    let broker_fee = parse_optional_fee(fields.broker_fee, "broker_fee")?;
    let tob_fee = parse_optional_fee(fields.tob_fee, "tob_fee")?;

    let executed_at = fields.executed_at.naive_utc();

    // fetch listing info, do not trust frontend
    let listing = sqlx::query!(
        r#"
        SELECT
            instrument_id AS "instrument_id!",
            currency_code AS "currency_code!"
        FROM listing
        WHERE id = ?1 AND delisted_at IS NULL
        "#,
        fields.listing_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let mut tx = pool.begin().await?;

    // broker_id is hardcoded to 1 (Re=bel) for MVP.
    // settlement_date and settlement_cash_id are deferred (cash tracking is out of scope).
    let trade_id = sqlx::query!(
        r#"
        INSERT INTO trade
            (broker_order_id, listing_id, broker_id, side, quantity, price, executed_at)
        VALUES
            (NULL, ?1, 1, 'BUY', ?2, ?3, ?4)
        "#,
        fields.listing_id,
        quantity_str,
        unit_price_str,
        executed_at,
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();

    // Re=bel always charges broker fees in EUR.
    if let Some(fee) = broker_fee {
        let fee_str = fee.to_string();
        sqlx::query!(
            r#"
            INSERT INTO trade_fee (trade_id, fee_type, amount, currency_code)
            VALUES (?1, 'BROKER', ?2, 'EUR')
            "#,
            trade_id,
            fee_str
        )
        .execute(&mut *tx)
        .await?;
    }

    // TOB is charged in the listing's own currency.
    if let Some(fee) = tob_fee {
        let fee_str = fee.to_string();
        sqlx::query!(
            r#"
            INSERT INTO trade_fee (trade_id, fee_type, amount, currency_code)
            VALUES (?1, 'TOB', ?2, ?3)
            "#,
            trade_id,
            fee_str,
            listing.currency_code,
        )
        .execute(&mut *tx)
        .await?;
    }

    // The lot records the acquisition fact. broker_id_at_acquisition matches the trade.
    sqlx::query!(
        r#"
        INSERT INTO lot
            (broker_id_at_acquisition, instrument_id, listing_id,
             source_trade_id, qty_at_acquisition, price_currency_code, price_per_unit)
        VALUES
            (1, ?1, ?2, ?3, ?4, ?5, ?6)
        "#,
        listing.instrument_id,
        fields.listing_id,
        trade_id,
        quantity_str,
        listing.currency_code,
        unit_price_str
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn buy(db: State<'_, Db>, fields: CreateBuyTradeInput) -> Result<(), AppError> {
    buy_core(&db.pool, fields).await
}

#[tauri::command]
#[specta::specta]
pub async fn import_buy_csv(db: State<'_, Db>, csv_content: String) -> Result<(), AppError> {
    // Set up the CSV reader to parse headers matching your struct fields
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All) // Clean up accidental whitespace around fields
        .from_reader(csv_content.as_bytes());

    // Loop through rows and pipe them into your core business logic
    for (index, result) in rdr.deserialize::<CreateBuyTradeInput>().enumerate() {
        let fields = result.map_err(|e| {
            AppError::Database(format!("CSV row {} parsing error: {}", index + 1, e))
        })?;

        buy_core(&db.pool, fields).await?;
    }

    Ok(())
}
