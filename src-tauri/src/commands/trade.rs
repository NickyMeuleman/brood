use crate::commands::tax::{SellComputation, compute_sell};
use crate::db::Db;
use crate::db::types::InstrumentType;
use crate::sync::backfill;
use crate::{AppError, EEA_DOMICILES, HttpClient, parse_decimal_external};
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
    pub tob_rate_hint: Option<TobRateHint>,
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
            i.instrument_type   AS "instrument_type!: InstrumentType",
            i.accumulating      AS "accumulating!",
            i.fsma_registered   AS "fsma_registered!",
            i.domicile,
            CASE WHEN (
                i.fund_family_id IS NOT NULL AND EXISTS (
                    SELECT 1 FROM instrument i2
                    WHERE i2.fund_family_id = i.fund_family_id
                    AND i2.id != i.id
                    AND i2.fsma_registered = 1
                )
            ) THEN 1 ELSE 0 END AS "fsma_registered_family"
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
        .map(|r| {
            let accumulating = r.accumulating != 0;
            let fsma_direct = r.fsma_registered != 0;
            let fsma_family = r.fsma_registered_family != 0;
            let tob_rate_hint = tob_rate_hint(
                &r.instrument_type,
                accumulating,
                fsma_direct,
                fsma_family,
                r.domicile.as_deref(),
            );
            ListingInfo {
                id: r.id,
                ticker: r.ticker,
                exchange_mic: r.exchange_mic,
                currency_code: r.currency_code,
                instrument_name: r.instrument_name,
                isin: r.isin,
                instrument_type: r.instrument_type,
                tob_rate_hint,
            }
        })
        .collect())
}

#[derive(Debug, Deserialize, Type)]
pub struct CreateBuyTradeInput {
    pub listing_id: i64,
    pub broker_id: i64,
    pub quantity: String,
    pub unit_price: String,
    pub executed_at: DateTime<Utc>,
    pub broker_fee: Option<String>,
    pub tob_fee: Option<String>,
}

async fn buy_core(pool: &Pool<Sqlite>, fields: CreateBuyTradeInput) -> Result<(), AppError> {
    let quantity = parse_decimal_external(&fields.quantity, "quantity")?;
    if quantity <= Decimal::ZERO {
        return Err(AppError::Validation("quantity must be positive".into()));
    }
    let quantity_str = quantity.to_string();
    let unit_price = parse_decimal_external(&fields.unit_price, "unit_price")?;
    if unit_price <= Decimal::ZERO {
        return Err(AppError::Validation("unit_price must be positive".into()));
    }
    let unit_price_str = unit_price.to_string();

    let parse_optional_fee =
        |raw: Option<String>, ctx: &'static str| -> Result<Option<Decimal>, AppError> {
            match raw {
                None => Ok(None),
                Some(v) if v.is_empty() || v == "0" => Ok(None),
                Some(v) => {
                    let d = parse_decimal_external(&v, ctx)?;
                    if d < Decimal::ZERO {
                        return Err(AppError::Validation(format!("{ctx} must be non-negative")));
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
    .ok_or(AppError::NotFound(format!(
        "Listing {} not found (or delisted)",
        fields.listing_id
    )))?;

    let mut tx = pool.begin().await?;

    // settlement_date and settlement_cash_id are deferred (cash tracking is out of scope).
    let trade_id = sqlx::query!(
        r#"
        INSERT INTO trade
            (broker_order_id, listing_id, broker_id, side, quantity, price, executed_at)
        VALUES
            (NULL, ?1, ?2, 'BUY', ?3, ?4, ?5)
        "#,
        fields.listing_id,
        fields.broker_id,
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

    // TOB is always charged in EUR.
    if let Some(fee) = tob_fee {
        let fee_str = fee.to_string();
        sqlx::query!(
            r#"
            INSERT INTO trade_fee (trade_id, fee_type, amount, currency_code)
            VALUES (?1, 'TOB', ?2, 'EUR')
            "#,
            trade_id,
            fee_str,
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
            (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        fields.broker_id,
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
pub async fn buy(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
    fields: CreateBuyTradeInput,
) -> Result<(), AppError> {
    let listing_id = fields.listing_id;
    buy_core(&db.pool, fields).await?;

    // try to backfill prices/FX-rates for this holding, not a hard error
    if let Err(e) = backfill(&db.pool, &http.client, listing_id).await {
        eprintln!("Post-buy sync for listing {listing_id} failed (non-fatal): {e}");
    }

    Ok(())
}

#[derive(Debug, Serialize, Type)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ImportRowOutcome {
    Success { row: usize },
    Error { row: usize, message: String },
}

#[tauri::command]
#[specta::specta]
pub async fn import_buy_csv(
    db: State<'_, Db>,
    csv_content: String,
) -> Result<Vec<ImportRowOutcome>, AppError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .trim(csv::Trim::All)
        .from_reader(csv_content.as_bytes());

    let mut outcomes = Vec::new();

    for (index, result) in rdr.deserialize::<CreateBuyTradeInput>().enumerate() {
        let row = index + 1;
        match result {
            Err(e) => outcomes.push(ImportRowOutcome::Error {
                row,
                message: format!("Could not parse row: {e}"),
            }),
            Ok(fields) => match buy_core(&db.pool, fields).await {
                Ok(()) => outcomes.push(ImportRowOutcome::Success { row }),
                Err(e) => outcomes.push(ImportRowOutcome::Error {
                    row,
                    message: e.to_string(),
                }),
            },
        }
    }

    Ok(outcomes)
}

#[derive(Debug, Deserialize, Type)]
pub struct CreateSellTradeInput {
    pub listing_id: i64,
    pub quantity: String,
    pub unit_price: String,
    pub executed_at: DateTime<Utc>,
    pub broker_fee: Option<String>,
    pub tob_fee: Option<String>,
}

/// Writes are gated entirely on `compute_sell`'s success: if the tax
/// computation errors, execution stops there and nothing is written.
pub async fn sell_core(
    pool: &Pool<Sqlite>,
    fields: CreateSellTradeInput,
) -> Result<SellComputation, AppError> {
    // broker id hardcoded to 1 for re=bel for now
    let computation = compute_sell(pool, &fields, 1).await?;

    let quantity = parse_decimal_external(&fields.quantity, "quantity")?;
    let quantity_str = quantity.to_string();
    let unit_price = parse_decimal_external(&fields.unit_price, "unit_price")?;
    let unit_price_str = unit_price.to_string();
    let executed_at = fields.executed_at.naive_utc();

    let parse_optional_fee =
        |raw: Option<String>, ctx: &'static str| -> Result<Option<Decimal>, AppError> {
            match raw {
                None => Ok(None),
                Some(v) if v.is_empty() || v == "0" => Ok(None),
                Some(v) => {
                    let d = parse_decimal_external(&v, ctx)?;
                    if d < Decimal::ZERO {
                        return Err(AppError::Validation(format!("{ctx} must be non-negative")));
                    }
                    Ok(Some(d))
                }
            }
        };
    let broker_fee = parse_optional_fee(fields.broker_fee, "broker_fee")?;
    let tob_fee = parse_optional_fee(fields.tob_fee, "tob_fee")?;

    let mut tx = pool.begin().await?;

    let trade_id = sqlx::query!(
        r#"
        INSERT INTO trade
            (broker_order_id, listing_id, broker_id, side, quantity, price, executed_at)
        VALUES
            (NULL, ?1, ?2, 'SELL', ?3, ?4, ?5)
        "#,
        fields.listing_id,
        // hardcoded to 1 for re=bel for now
        1,
        quantity_str,
        unit_price_str,
        executed_at,
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();

    // Both fee types are always EUR — TOB by law and re=bel only charges EUR too.
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

    if let Some(fee) = tob_fee {
        let fee_str = fee.to_string();
        sqlx::query!(
            r#"
            INSERT INTO trade_fee (trade_id, fee_type, amount, currency_code)
            VALUES (?1, 'TOB', ?2, 'EUR')
            "#,
            trade_id,
            fee_str
        )
        .execute(&mut *tx)
        .await?;
    }

    let mut total_allocated = Decimal::ZERO;
    for alloc in &computation.allocations {
        total_allocated += alloc.quantity;
        let quantity_alloc_str = alloc.quantity.to_string();

        let sell_allocation_id = sqlx::query!(
            r#"
            INSERT INTO sell_allocation (sell_trade_id, origin_lot_id, quantity)
            VALUES (?1, ?2, ?3)
            "#,
            trade_id,
            alloc.origin_lot_id,
            quantity_alloc_str
        )
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();

        if let Some(tax) = &alloc.tax {
            let sale_price_str = tax.sale_price_eur.to_string();
            let buy_price_str = tax.taxable_buy_price_eur.to_string();
            let taxable_gain_str = tax.taxable_gain_eur.to_string();
            let computed_at = Utc::now().naive_utc();
            let tax_year = computation
                .tax_year
                .expect("tax_year is always Some when subject_to_cgt (and thus tax) is Some");

            let (fotomoment, historical, method, snapshot_id) = match &tax.pre2026_cost_basis {
                Some(basis) => (
                    Some(basis.fotomoment_cost_eur.to_string()),
                    Some(basis.historical_cost_eur.to_string()),
                    Some(basis.method.clone()),
                    Some(basis.tax_snapshot_2025_id),
                ),
                None => (None, None, None, None),
            };

            sqlx::query!(
                r#"
                INSERT INTO tax_sell_allocation (
                    sell_allocation_id, sale_price_eur, buy_price_eur, taxable_gain_eur,
                    tax_year, computed_at, sale_fx_rate_id, buy_fx_rate_id,
                    fotomoment_cost_eur, historical_cost_eur,
                    pre2026_cost_basis_method, tax_snapshot_2025_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                "#,
                sell_allocation_id,
                sale_price_str,
                buy_price_str,
                taxable_gain_str,
                tax_year,
                computed_at,
                tax.sale_fx_rate_id,
                tax.buy_fx_rate_id,
                fotomoment,
                historical,
                method,
                snapshot_id,
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    debug_assert_eq!(
        total_allocated, quantity,
        "sum of sell_allocation quantities must equal trade.quantity"
    );

    tx.commit().await?;
    Ok(computation)
}

#[tauri::command]
#[specta::specta]
pub async fn preview_sell(
    db: State<'_, Db>,
    fields: CreateSellTradeInput,
) -> Result<SellComputation, AppError> {
    // hardcoded 1 for re=bel for now
    compute_sell(&db.pool, &fields, 1).await
}

#[tauri::command]
#[specta::specta]
pub async fn sell(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
    fields: CreateSellTradeInput,
) -> Result<SellComputation, AppError> {
    let listing_id = fields.listing_id;
    let computation = sell_core(&db.pool, fields).await?;
    // non-fatal backfill-on-success
    if let Err(e) = backfill(&db.pool, &http.client, listing_id).await {
        eprintln!("Post-sell sync for listing {listing_id} failed (non-fatal): {e}");
    }
    Ok(computation)
}

#[derive(Debug, Serialize, Type)]
pub struct TobRateHint {
    buy: Decimal,
    sell: Decimal,
}
impl TobRateHint {
    fn monorate(rate: Decimal) -> Self {
        Self {
            buy: rate,
            sell: rate,
        }
    }
}

// https://curvo.eu/nl/artikel/beurstaks-tob
fn tob_rate_hint(
    instrument_type: &InstrumentType,
    accumulating: bool,
    fsma_registered_direct: bool,
    family_fsma_registered: bool,
    domicile: Option<&str>,
) -> Option<TobRateHint> {
    match instrument_type {
        InstrumentType::Stock => Some(TobRateHint::monorate(Decimal::new(35, 4))),
        InstrumentType::Etf => {
            let fsma = fsma_registered_direct || family_fsma_registered;
            let eer = domicile.is_some_and(|v| EEA_DOMICILES.contains(&v));
            let rate = match (fsma, eer, accumulating) {
                (true, _, false) => TobRateHint::monorate(Decimal::new(12, 4)),
                (true, _, true) => TobRateHint::monorate(Decimal::new(132, 4)),
                (false, true, _) => TobRateHint::monorate(Decimal::new(12, 4)),
                (_, false, _) => TobRateHint::monorate(Decimal::new(35, 4)),
            };
            Some(rate)
        }
        InstrumentType::Fund => {
            let rate = match accumulating {
                true => TobRateHint {
                    buy: Decimal::ZERO,
                    sell: Decimal::new(132, 4),
                },
                false => TobRateHint {
                    buy: Decimal::ZERO,
                    sell: Decimal::ZERO,
                },
            };
            Some(rate)
        }
        InstrumentType::Bond => Some(TobRateHint {
            buy: Decimal::ZERO,
            sell: Decimal::new(12, 4),
        }),
        InstrumentType::Other => None,
    }
}

fn d(v: i64) -> Decimal {
    Decimal::new(v, 0)
}

fn per_slice(amount: Decimal, fee: Decimal) -> Decimal {
    let slices = (amount / d(10_000)).ceil();
    fee * slices
}

#[tauri::command]
#[specta::specta]
pub fn broker_fee_hint(
    broker: String,
    quantity: String,
    unit_price: String,
    instrument_type: InstrumentType,
    mic: String,
    fx_rate: String,
) -> Result<Option<Decimal>, AppError> {
    let amount = parse_decimal_external(&quantity, "quantity")?
        * parse_decimal_external(&unit_price, "unit price")?
        * parse_decimal_external(&fx_rate, "fx rate")?;
    Ok(rebel_broker_fee(&instrument_type, amount, &mic))
}

fn rebel_broker_fee(
    instrument_type: &InstrumentType,
    amount: Decimal,
    mic: &str,
) -> Option<Decimal> {
    match (instrument_type, mic) {
        // Euronext Brussels
        (InstrumentType::Stock, "XBRU") if amount <= d(2500) => Some(d(3)),
        (InstrumentType::Stock, "XBRU") => Some(per_slice(amount, d(10))),
        // Euronext Paris & Amsterdam
        (InstrumentType::Stock, "XPAR" | "XAMS") if amount <= d(1000) => Some(d(3)),
        (InstrumentType::Stock, "XPAR" | "XAMS") if amount <= d(2500) => Some(d(6)),
        (InstrumentType::Stock, "XPAR" | "XAMS") => Some(per_slice(amount, d(10))),
        // USA
        (InstrumentType::Stock, "XNAS" | "XNYS" | "XASE") if amount <= d(1000) => Some(d(5)),
        (InstrumentType::Stock, "XNAS" | "XNYS" | "XASE") if amount <= d(2500) => Some(d(9)),
        (InstrumentType::Stock, "XNAS" | "XNYS" | "XASE") => Some(per_slice(amount, d(12))),
        // Germany
        (InstrumentType::Stock, "XETR" | "XFRA") if amount <= d(2500) => Some(d(12)),
        (InstrumentType::Stock, "XETR" | "XFRA") => Some(per_slice(amount, d(15))),

        // Euronext Brussels
        (InstrumentType::Etf, "XBRU") if amount <= d(250) => Some(d(1)),
        (InstrumentType::Etf, "XBRU") if amount <= d(1000) => Some(d(2)),
        (InstrumentType::Etf, "XBRU") if amount <= d(2500) => Some(d(3)),
        (InstrumentType::Etf, "XBRU") => Some(per_slice(amount, d(10))),
        // Euronext Paris & Amsterdam
        (InstrumentType::Etf, "XPAR" | "XAMS") if amount <= d(250) => Some(d(1)),
        (InstrumentType::Etf, "XPAR" | "XAMS") if amount <= d(1000) => Some(d(2)),
        (InstrumentType::Etf, "XPAR" | "XAMS") if amount <= d(2500) => Some(d(6)),
        (InstrumentType::Etf, "XPAR" | "XAMS") => Some(per_slice(amount, d(10))),
        // Germany
        (InstrumentType::Etf, "XETR" | "XFRA") if amount <= d(2500) => Some(d(12)),
        (InstrumentType::Etf, "XETR" | "XFRA") => Some(per_slice(amount, d(15))),

        _ => None,
    }
}
