use crate::db::Db;
use crate::sync::fx::{FXSyncOutcome, force_sync_all_fx, force_sync_one_currency, sync_all_fx};
use crate::sync::prices::{
    PriceSyncOutcome, force_sync_all_prices, force_sync_one_listing, sync_all_prices,
};
use crate::{AppError, HttpClient};
use std::time::Duration;
use tauri::State;
use tokio;

/// Applied to every sync command so a stalled network request can never
/// hang a command indefinitely. Sits above the client-level timeout in
/// lib.rs as a backstop, not the primary guard.
const SYNC_TIMEOUT: Duration = Duration::from_secs(15);

/// cleaner implementation than a Trait ... for now
async fn with_timeout<T>(
    label: impl Into<String>,
    fut: impl Future<Output = Result<T, AppError>>,
) -> Result<T, AppError> {
    match tokio::time::timeout(SYNC_TIMEOUT, fut).await {
        Ok(result) => result,
        Err(_) => Err(AppError::Timeout(format!(
            "{} timed out after {}s",
            label.into(),
            SYNC_TIMEOUT.as_secs()
        ))),
    }
}

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
    let fut = async {
        let (fx_outcomes, prices_outcomes) = tokio::try_join!(
            sync_all_fx(&db.pool, &http.client),
            sync_all_prices(&db.pool, &http.client)
        )?;

        Ok(SyncOutcomes {
            fx: fx_outcomes,
            prices: prices_outcomes,
        })
    };
    with_timeout("Sync", fut).await
}

#[tauri::command]
#[specta::specta]
pub async fn sync_prices(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
) -> Result<Vec<PriceSyncOutcome>, AppError> {
    let fut = async {
        let outcomes = sync_all_prices(&db.pool, &http.client).await?;
        Ok(outcomes)
    };
    with_timeout("Price sync", fut).await
}

#[tauri::command]
#[specta::specta]
pub async fn sync_fx(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
) -> Result<Vec<FXSyncOutcome>, AppError> {
    let fut = async {
        let outcomes = sync_all_fx(&db.pool, &http.client).await?;
        Ok(outcomes)
    };
    with_timeout("FX sync", fut).await
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
    with_timeout(
        format!("Price sync for {ticker}"),
        force_sync_one_listing(&db.pool, &http.client, listing_id, mic, ticker),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn force_update_all_prices(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
) -> Result<Vec<PriceSyncOutcome>, AppError> {
    with_timeout(
        "Full price sync",
        force_sync_all_prices(&db.pool, &http.client),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn force_update_one_currency_fx(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
    currency: String,
) -> Result<FXSyncOutcome, AppError> {
    with_timeout(
        format!("FX sync for {currency}"),
        force_sync_one_currency(&db.pool, &http.client, currency),
    )
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn force_update_all_fx(
    db: State<'_, Db>,
    http: State<'_, HttpClient>,
) -> Result<Vec<FXSyncOutcome>, AppError> {
    with_timeout("Full FX sync", force_sync_all_fx(&db.pool, &http.client)).await
}
