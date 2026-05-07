use chrono::{NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
struct ChartResponse {
    chart: Chart,
}

#[derive(Debug, Serialize, Deserialize)]
struct Chart {
    result: Option<Vec<ChartResult>>,
    error: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct ChartResult {
    pub meta: Meta,
    pub timestamp: Option<Vec<i64>>,
    pub indicators: Indicators,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub currency: String,
    pub symbol: String,
    pub exchange_name: String,
    pub full_exchange_name: String,
    pub instrument_type: String,
    pub first_trade_date: i64,
    pub regular_market_time: i64,
    pub has_pre_post_market_data: bool,
    pub gmtoffset: i64,
    pub timezone: String,
    pub exchange_timezone_name: String,
    pub regular_market_price: Decimal,
    pub fifty_two_week_high: Decimal,
    pub fifty_two_week_low: Decimal,
    pub regular_market_day_high: Decimal,
    pub regular_market_day_low: Decimal,
    pub regular_market_volume: i64,
    pub long_name: String,
    pub short_name: String,
    pub chart_previous_close: Decimal,
    pub price_hint: i32,
    pub current_trading_period: TradingPeriods,
    pub data_granularity: String,
    pub range: String,
    pub valid_ranges: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradingPeriods {
    pub pre: TradingPeriod,
    pub regular: TradingPeriod,
    pub post: TradingPeriod,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradingPeriod {
    pub timezone: String,
    pub start: i64,
    pub end: i64,
    pub gmtoffset: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Indicators {
    pub quote: Vec<Quote>,
    pub adjclose: Vec<AdjClose>,
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
    pub adjclose: Vec<Decimal>,
}

/// Returns an empty string for US exchanges (no suffix needed).
pub fn yahoo_suffix(exchange_mic: &str) -> Result<&'static str, Error> {
    match exchange_mic {
        "XNAS" | "XNYS" | "NYSE" | "ARCX" => Ok(""), // Murica
        "XAMS" => Ok(".AS"),                         // Amsterdam
        "XPAR" => Ok(".PA"),                         // Paris
        "XBRU" => Ok(".BR"),                         // Brussels
        "XLIS" => Ok(".LS"),                         // Lisbon
        "XETR" => Ok(".DE"),                         // Frankfurt - Xetra
        "XFRA" => Ok(".F"),                          // Frankfurt - Borse
        "XSTU" => Ok(".SG"),                         // Stuttgart
        "XLON" => Ok(".L"),                          // London
        "XSWX" => Ok(".SW"),                         // Zurich
        "XMIL" => Ok(".MI"),                         // Milan
        "XSTO" => Ok(".ST"),                         // Stockholm
        "XCSE" => Ok(".CO"),                         // Copenhagen
        "XHEL" => Ok(".HE"),                         // Helsinki
        "XOSL" => Ok(".OL"),                         // Oslo
        other => Err(Error::Parse(format!(
            "Unknown exchange MIC '{other}': add it to yahoo_suffix() before syncing"
        ))),
    }
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
    exchange_mic: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<PriceBar>, Error> {
    let suffix = yahoo_suffix(exchange_mic)?;
    let symbol = format!("{ticker}{suffix}");

    // Use noon UTC to avoid DST edge cases when converting NaiveDate → timestamp.
    // Obligatory Tom Scott timezone video
    let period1 = Utc
        .from_utc_datetime(
            &from
                .and_hms_opt(12, 0, 0)
                .ok_or(Error::Parse("Invalid 'from' date/time".into()))?,
        )
        .timestamp();
    let period2 = Utc
        .from_utc_datetime(
            &to.and_hms_opt(12, 0, 0)
                .ok_or(Error::Parse("Invalid 'to' date/time".into()))?,
        )
        .timestamp();

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

    if quote.open.len() != timestamps.len()
        || quote.high.len() != timestamps.len()
        || quote.low.len() != timestamps.len()
        || quote.close.len() != timestamps.len()
    {
        return Err(Error::Parse(
            "Mismatched list lengths in Yahoo response".into(),
        ));
    }

    let exchange_tz: Tz = result
        .meta
        .exchange_timezone_name
        .parse()
        .map_err(|_| Error::Parse("Invalid exchange timezone".into()))?;

    let mut bars = Vec::new();
    for (i, ts) in timestamps.iter().enumerate() {
        // Yahoo returns timestamps at market open time in the exchange's local timezone
        let datetime = exchange_tz
            .timestamp_opt(*ts, 0)
            .single()
            .ok_or_else(|| Error::Parse("Invalid timestamp".into()))?;

        bars.push(PriceBar {
            date: datetime.date_naive(),
            open: quote.open[i].ok_or(Error::MissingField("open"))?,
            high: quote.high[i].ok_or(Error::MissingField("high"))?,
            low: quote.low[i].ok_or(Error::MissingField("low"))?,
            close: quote.close[i].ok_or(Error::MissingField("close"))?,
            volume: quote.volume[i].unwrap_or(0),
        });
    }

    dbg!(symbol, period1, period2, url, &bars);

    Ok(bars)
}
