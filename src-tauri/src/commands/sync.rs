use crate::db::Db;
use crate::sync::fx::{
    FXSyncOutcome, FxSyncTask, get_full_fx_sync_tasks, run_fx_sync_tasks, sync_all_fx,
    sync_one_currency,
};
use crate::sync::prices::{
    PriceSyncOutcome, PriceSyncTask, get_full_price_sync_tasks, run_price_sync_tasks,
    sync_all_prices, sync_one_listing,
};
use crate::{AppError, HttpClient, mic_timezone};
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
    let tz = mic_timezone(&mic).map_err(|e| AppError::Internal(e.to_string()))?;
    let to = Utc::now()
        .with_timezone(&tz)
        .date_naive()
        .pred_opt()
        .expect("valid date");

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
    let tasks = get_full_price_sync_tasks(&db.pool).await?;
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
    let tasks = get_full_fx_sync_tasks(&db.pool).await?;
    let outcomes = run_fx_sync_tasks(&db.pool, &http.client, tasks).await;
    Ok(outcomes)
}
