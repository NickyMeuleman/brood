use crate::{
    sync::yahoo::{ChartResponse, Error, Meta},
    yahoo_suffix,
};
use reqwest::Client;

pub async fn get_listing_meta(client: &Client, ticker: &str, mic: &str) -> Result<Meta, Error> {
    let suffix = yahoo_suffix(mic)?;
    let symbol = format!("{ticker}{suffix}");
    let url =
        format!("https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?range=1d&interval=1d");

    let response: ChartResponse = client
        .get(&url)
        // Yahoo sometimes rejects requests without a browser-like User-Agent
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await?
        .json()
        .await?;

    if let Some(err) = response.chart.error {
        return Err(Error::Parse(format!("Yahoo returned error: {err}")));
    }

    let result = response
        .chart
        .result
        .and_then(|r| r.into_iter().next())
        .ok_or_else(|| Error::Parse(format!("No listing found for {ticker} on {mic}")))?;

    Ok(result.meta)
}
