use chrono::{NaiveDate, TimeZone, Utc};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta::Type;
use thiserror::Error;

use crate::{mic_timezone, yahoo_suffix};

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Unexpected response shape: {0}")]
    Parse(String),

    #[error("Missing field: {0}")]
    MissingField(&'static str),

    #[error("No data returned for {ticker} ({from} – {to})")]
    NoData {
        ticker: String,
        from: NaiveDate,
        to: NaiveDate,
    },

    #[error("Unknown exchange configuration: {0}")]
    UnknownMic(String),
}

#[derive(Debug, Clone)]
pub struct PriceBar {
    pub date: NaiveDate,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChartResponse {
    pub chart: Chart,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chart {
    pub result: Option<Vec<ChartResult>>,
    pub error: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct ChartResult {
    pub meta: Meta,
    pub timestamp: Option<Vec<i64>>,
    pub indicators: Indicators,
}

// only fields that are necessary are not an Option or defaulted
#[derive(Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub currency: Option<String>,
    pub symbol: String,
    pub exchange_name: String,
    pub full_exchange_name: String,
    pub instrument_type: Option<String>,
    pub first_trade_date: Option<i64>,
    pub regular_market_time: Option<i64>,
    #[serde(default)]
    pub has_pre_post_market_data: bool,
    pub gmtoffset: Option<i64>,
    pub timezone: Option<String>,
    pub exchange_timezone_name: String,
    pub regular_market_price: Option<Decimal>,
    pub fifty_two_week_high: Option<Decimal>,
    pub fifty_two_week_low: Option<Decimal>,
    pub regular_market_day_high: Option<Decimal>,
    pub regular_market_day_low: Option<Decimal>,
    pub regular_market_volume: Option<i64>,
    pub long_name: Option<String>,
    pub short_name: Option<String>,
    pub chart_previous_close: Option<Decimal>,
    pub price_hint: Option<i32>,
    pub current_trading_period: Option<TradingPeriods>,
    #[serde(default)]
    pub data_granularity: String,
    #[serde(default)]
    pub range: String,
    #[serde(default)]
    pub valid_ranges: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct TradingPeriods {
    pub pre: TradingPeriod,
    pub regular: TradingPeriod,
    pub post: TradingPeriod,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct TradingPeriod {
    pub timezone: String,
    pub start: i64,
    pub end: i64,
    pub gmtoffset: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Indicators {
    pub quote: Vec<Quote>,
    #[serde(default)]
    pub adjclose: Option<Vec<AdjClose>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Quote {
    open: Vec<Option<Decimal>>,
    close: Vec<Option<Decimal>>,
    high: Vec<Option<Decimal>>,
    volume: Vec<Option<u64>>,
    low: Vec<Option<Decimal>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdjClose {
    adjclose: Option<Vec<Option<Decimal>>>,
}

fn make_bar(quote: &Quote, i: usize, date: NaiveDate) -> Result<PriceBar, Error> {
    let get_val =
        |vec: &[Option<Decimal>], i: usize, field: &'static str| -> Result<Decimal, Error> {
            vec.get(i)
                .and_then(|v| *v)
                .ok_or(Error::MissingField(field))
        };

    // if any part of the bar was null in JSON, skip that day but don't skip the entire fetch
    Ok(PriceBar {
        date,
        open: get_val(&quote.open, i, "open")?,
        high: get_val(&quote.high, i, "high")?,
        low: get_val(&quote.low, i, "low")?,
        close: get_val(&quote.close, i, "close")?,
        volume: quote.volume.get(i).and_then(|v| *v).unwrap_or(0),
    })
}

/// Fetch daily OHLCV bars for a single listing from Yahoo Finance.
///
/// `ticker`       — the ticker as stored in your `listing` table (e.g. "SWRD")
/// `exchange_mic` — the MIC as stored in your `listing` table (e.g. "XAMS")
/// `from`         — first date to fetch (inclusive)
/// `to`           — last date to fetch (inclusive)
///
/// Returns an empty Vec if Yahoo has no data for the requested window
/// (e.g. the window falls entirely on weekends/holidays).
pub async fn fetch_prices(
    client: &Client,
    ticker: &str,
    mic: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<PriceBar>, Error> {
    let exchange_tz = mic_timezone(mic)?;
    let suffix = yahoo_suffix(mic)?;
    let symbol = format!("{ticker}{suffix}");

    // Yahoo expects these timestamps in UTC ... I think (no official docs sadly)
    // Obligatory Tom Scott timezone video
    // period1 is inclusive, period2 is exclusive
    let period1 = Utc
        .from_utc_datetime(
            &from
                .and_hms_opt(0, 0, 0)
                .ok_or(Error::Parse("Invalid 'from' date/time".into()))?,
        )
        .timestamp();
    // too wide by choice to make certain yahoo doesn't drop the last finished day because the to
    // timestamp didn't fall in exchange opening hours. timezone stuff is fun
    let period2 = Utc::now().timestamp();

    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}\
         ?period1={period1}&period2={period2}&interval=1d"
    );

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

    let results = match response.chart.result {
        Some(res) if !res.is_empty() => res,
        _ => return Ok(Vec::new()), // no data for this window; not an error
    };

    let result = &results
        .first()
        .ok_or_else(|| Error::Parse("Missing API result".into()))?;
    let quote = result
        .indicators
        .quote
        .first()
        .ok_or_else(|| Error::Parse("Missing quote indicators".into()))?;
    let timestamps = result
        .timestamp
        .as_ref()
        .ok_or_else(|| Error::MissingField("timestamp"))?;

    // double check timezone
    let yahoo_tz_name = &result.meta.exchange_timezone_name;
    let yahoo_tz = yahoo_tz_name.parse().map_err(|_| {
        Error::Parse(format!(
            "Unknown Yahoo timezone '{yahoo_tz_name}' is invalid"
        ))
    })?;

    if exchange_tz != yahoo_tz {
        return Err(Error::Parse("Inconsistent timezone info".to_string()));
    }

    let mut bars = Vec::new();
    for (i, ts) in timestamps.iter().enumerate() {
        // Yahoo returns timestamps at market open time in the exchange's local timezone
        let datetime = exchange_tz
            .timestamp_opt(*ts, 0)
            .single()
            .ok_or_else(|| Error::Parse("Invalid timestamp".into()))?;
        let date = datetime.date_naive();

        // ignore bars before from (apparently yahoo sometimes returns some of those, no idea why)
        // ignore bars after to (prevent saving in-progress bars)
        if date < from || date > to {
            continue;
        }

        match make_bar(quote, i, date) {
            Ok(bar) => bars.push(bar),
            Err(e) => eprintln!("Skipping bar for {date}: {e}"),
        }
    }

    Ok(bars)
}
