use crate::db::Db;
use crate::sync::fx::{run_fx_sync_tasks, sync_all_fx, sync_one_currency, FXSyncOutcome, FxSyncTask};
use crate::sync::prices::{
    run_price_sync_tasks, sync_one_listing, sync_all_prices, PriceSyncOutcome, PriceSyncTask,
};
use crate::{mic_timezone, AppError, HttpClient};
use chrono::{NaiveDate, Utc};
use std::time::Duration;
use tauri::State;
use tokio;

#[derive(Debug, serde::Serialize, specta::Type)]
#[serde(tag = "status", rename_all = "camelCase")]
pub struct SyncOutcomes {
    fx: Vec<FXSyncOutcome>,
    prices: Vec<PriceSyncOutcome>,
}

#[tauri::command]
#[specta::specta]
pub async fn sync(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
) -> Result<SyncOutcomes, AppError> {
    let sync_task = async {
        let (fx_outcomes, prices_outcomes) = tokio::try_join!(
            sync_all_fx(&db.pool, &http.client),
            sync_all_prices(&db.pool, &http.client)
        )?;

        Ok(SyncOutcomes {
            fx: fx_outcomes,
            prices: prices_outcomes,
        })
    };

    match tokio::time::timeout(Duration::from_secs(15), sync_task).await {
        Ok(result) => result,
        Err(_) => Err(AppError::Timeout("Sync timed out after 15s".into())),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn sync_prices(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
) -> Result<Vec<PriceSyncOutcome>, AppError> {
    let outcomes = sync_all_prices(&db.pool, &http.client).await?;
    Ok(outcomes)
}

#[tauri::command]
#[specta::specta]
pub async fn sync_fx(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
) -> Result<Vec<FXSyncOutcome>, AppError> {
    let outcomes = sync_all_fx(&db.pool, &http.client).await?;
    Ok(outcomes)
}

#[tauri::command]
#[specta::specta]
pub async fn force_update_one_listing_prices(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
    listing_id: i64,
    mic: String,
    ticker: String,
) -> Result<PriceSyncOutcome, AppError> {
    let tz = mic_timezone(&mic).map_err(|_| AppError::Internal)?;
    let to = Utc::now()
        .with_timezone(&tz)
        .date_naive()
        .pred_opt()
        .unwrap();

    // get earliest data, fallback to yesterday if not found
    let from = sqlx::query_scalar!(
        r#"
        SELECT MIN(COALESCE(DATE(t.executed_at), DATE(ca.effective_date))) as "d: NaiveDate"
        FROM lot l
        LEFT JOIN trade t ON t.id = l.source_trade_id
        LEFT JOIN corporate_action ca ON ca.id = l.source_ca_id
        WHERE l.listing_id = ?
        "#,
        listing_id
    )
    .fetch_one(&db.pool)
    .await?
    .unwrap_or(to);

    let task = PriceSyncTask {
        listing_id,
        ticker: ticker.clone(),
        mic,
        from,
        to,
    };

    let added = sync_one_listing(&db.pool, &http.client, &task).await?;
    Ok(PriceSyncOutcome::Success { ticker, added })
}

#[tauri::command]
#[specta::specta]
pub async fn force_update_all_prices(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
) -> Result<Vec<PriceSyncOutcome>, AppError> {
    // 1. Get every active listing that has at least one lot
    let rows = sqlx::query!(
        r#"
        SELECT 
            li.id as "id!", li.ticker as "ticker!", li.exchange_mic as "mic!",
            (
                SELECT MIN(COALESCE(DATE(t.executed_at), DATE(ca.effective_date)))
                FROM lot l
                LEFT JOIN trade t ON t.id = l.source_trade_id
                LEFT JOIN corporate_action ca ON ca.id = l.source_ca_id
                WHERE l.listing_id = li.id
            )                   AS "earliest: NaiveDate"
        FROM listing li
        WHERE EXISTS (SELECT 1 FROM lot l WHERE l.listing_id = li.id)
          AND li.delisted_at IS NULL
        "#
    )
    .fetch_all(&db.pool)
    .await?;

    let tasks = rows
        .into_iter()
        .map(|row| {
            let tz = mic_timezone(&row.mic).map_err(|_| AppError::Internal)?;
            let to = Utc::now()
                .with_timezone(&tz)
                .date_naive()
                .pred_opt()
                .expect("valid date");
            let from = row.earliest.unwrap_or(to);
            Ok(PriceSyncTask {
                listing_id: row.id,
                ticker: row.ticker,
                mic: row.mic,
                from,
                to,
            })
        })
        .collect::<Result<Vec<PriceSyncTask>, AppError>>()?;
    let outcomes = run_price_sync_tasks(&db.pool, &http.client, tasks).await;

    Ok(outcomes)
}

#[tauri::command]
#[specta::specta]
pub async fn force_update_one_currency_fx(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
    currency: String,
) -> Result<FXSyncOutcome, AppError> {
    let to = Utc::now().date_naive().pred_opt().expect("valid date");
    let from = sqlx::query_scalar!(
        r#"
        SELECT MIN(COALESCE(DATE(t.executed_at), DATE(ca.effective_date))) AS "d: NaiveDate"
        FROM lot l
        LEFT JOIN listing li ON li.id = l.listing_id
        LEFT JOIN trade t    ON t.id  = l.source_trade_id
        LEFT JOIN corporate_action ca ON ca.id = l.source_ca_id
        WHERE li.currency_code = ?
        "#,
        currency
    )
    .fetch_one(&db.pool)
    .await?
    .unwrap_or(to);

    let task = FxSyncTask {
        currency: currency.clone(),
        from,
        to,
    };

    let added = sync_one_currency(&db.pool, &http.client, &task).await?;
    Ok(FXSyncOutcome::Success { currency, added })
}

#[tauri::command]
#[specta::specta]
pub async fn force_update_all_fx(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
) -> Result<Vec<FXSyncOutcome>, AppError> {
    let to = Utc::now().date_naive().pred_opt().expect("valid date");

    let rows = sqlx::query!(
        r#"
        SELECT
            li.currency_code AS "currency!",
            MIN(COALESCE(DATE(t.executed_at), DATE(ca.effective_date)))
                AS "earliest: NaiveDate"
        FROM lot l
        LEFT JOIN listing li            ON li.id  = l.listing_id
        LEFT JOIN lot_close lc          ON lc.lot_id = l.id
        LEFT JOIN trade t               ON t.id   = l.source_trade_id
        LEFT JOIN corporate_action ca   ON ca.id  = l.source_ca_id
        WHERE li.currency_code != 'EUR'
          AND li.delisted_at IS NULL
          AND lc.lot_id IS NULL
        GROUP BY li.currency_code
        "#
    )
    .fetch_all(&db.pool)
    .await?;

    let tasks = rows
        .into_iter()
        .map(|row| {
            let from = row.earliest.unwrap_or(to);
            FxSyncTask {
                currency: row.currency.clone(),
                from,
                to,
            }
        })
        .collect();
    let outcomes = run_fx_sync_tasks(&db.pool, &http.client, tasks).await;

    Ok(outcomes)
}
