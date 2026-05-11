use crate::db::Db;
use crate::sync::prices::{
    get_price_sync_tasks, sync_all_prices, sync_one_listing, PriceSyncTask, SyncOutcome,
};
use crate::AppError;
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn sync_market_data(db: State<'_, Db>) -> Result<Vec<SyncOutcome>, AppError> {
    let client = reqwest::Client::new();
    let outcomes = sync_all_prices(&db.pool, &client).await?;
    // let tasks = get_price_sync_tasks(&db.pool).await?;
    // for task in tasks {
    //     if task.ticker == "SWRD" {
    //         match sync_one_listing(&db.pool, &client, &task).await {
    //             Ok(n) => {
    //                 dbg!("Added", n);
    //             }
    //             Err(e) => {
    //                 dbg!(e);
    //             }
    //         }
    //     }
    // }
    dbg!(&outcomes);
    Ok(outcomes)
}
