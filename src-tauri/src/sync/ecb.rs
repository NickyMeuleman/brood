use chrono::NaiveDate;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::Deserialize;
use thiserror::Error;

use crate::db::types::FxRateSource;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Response parse error: {0}")]
    Parse(String),

    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },
}

#[derive(Deserialize, Debug)]
struct RateRecord {
    date: NaiveDate,
    base: String,
    quote: String,
    rate: Decimal,
}
#[derive(Debug)]
pub struct Rate {
    pub date: NaiveDate,
    pub currency: String,
    pub rate_to_eur: Decimal,
    pub source: FxRateSource,
}

#[derive(Deserialize, Debug)]
struct ApiError {
    pub message: String,
}

pub async fn fetch_rates(
    client: &Client,
    currency: &str,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<Rate>, Error> {
    let url = format!(
        "https://api.frankfurter.dev/v2/rates?base={currency}&quotes=EUR&from={from}&to={to}&providers=ECB",
    );
    let response = client.get(&url).send().await?;
    let status = response.status();
    let bytes = response.bytes().await?;

    if !status.is_success() {
        let err: ApiError = serde_json::from_slice(&bytes)
            .map_err(|_| Error::Parse(format!("HTTP {status} with unparseable error body")))?;
        return Err(Error::Api {
            status: status.as_u16(),
            message: err.message,
        });
    }

    let records: Vec<RateRecord> =
        serde_json::from_slice(&bytes).map_err(|e| Error::Parse(e.to_string()))?;

    records
        .into_iter()
        .map(|r| {
            if r.base != currency {
                return Err(Error::Parse(format!(
                    "expected base={currency}, got base={} on {}",
                    r.base, r.date
                )));
            }
            if r.quote != "EUR" {
                return Err(Error::Parse(format!(
                    "expected quote=EUR, got quote={} on {}",
                    r.quote, r.date
                )));
            }
            if r.rate <= Decimal::ZERO {
                return Err(Error::Parse(format!(
                    "non-positive rate {} for {currency}/EUR on {}",
                    r.rate, r.date
                )));
            }
            Ok(Rate {
                date: r.date,
                currency: r.base,
                rate_to_eur: r.rate,
                source: FxRateSource::Ecb,
            })
        })
        .collect()
}
