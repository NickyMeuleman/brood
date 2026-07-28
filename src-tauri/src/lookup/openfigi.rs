use crate::db::types::InstrumentType;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;

use crate::isin;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Response parse error: {0}")]
    Parse(String),

    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },
}

/// Free/unauthenticated OpenFIGI requests are capped at 10 mapping jobs per
/// request (100 with an API key — see README). Chunk rather than raise this.
const MAX_JOBS_PER_REQUEST: usize = 10;
const DELAY_BETWEEN_CHUNKS: Duration = Duration::from_millis(250);

#[derive(Deserialize, Debug)]
struct ApiError {
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FigiRequest<'a> {
    pub id_type: &'a str,
    pub id_value: &'a str,
    pub mic_code: &'a str,
}

#[derive(Deserialize, Serialize, Debug, Type)]
#[serde(rename_all = "camelCase")]
struct FigiData {
    pub figi: String,
    pub name: Option<String>,
    pub ticker: Option<String>,
    pub exch_code: Option<String>,
    #[serde(rename = "compositeFIGI")]
    pub composite_figi: Option<String>,
    pub security_type: Option<String>,
    pub market_sector: Option<String>,
    #[serde(rename = "shareClassFIGI")]
    pub share_class_figi: Option<String>,
    pub security_type_2: Option<String>,
    pub security_description: Option<String>,
}

#[derive(Deserialize, Debug)]
struct FigiResponse {
    pub data: Option<Vec<FigiData>>,
    // a job with no matches has a warning key
    #[allow(dead_code)]
    // not surfaced yet, use in future diagnostics (like in the SyncOutcome types)
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Type)]
pub struct ListingCandidate {
    pub mic: String,
    pub ticker: String,
    pub name: String,
    /// OpenFIGI's own security type (e.g. "Common Stock", "ETP") — a rough
    /// hint for instrument_type, not a reliable one-to-one mapping.
    pub security_type: Option<String>,
    /// A finer classification alongside security_type (e.g. "Mutual Fund"
    /// for an ETF) — same caveat.
    pub security_type_2: Option<String>,
    /// Bloomberg's coarse asset-class bucket (e.g. "Equity", "Govt", "Corp")
    /// — a rough stock-vs-bond signal, nothing more precise than that.
    pub market_sector: Option<String>,
}

pub async fn search_isin_listings(
    client: &Client,
    isin: &str,
    mics: &[&str],
) -> Result<Vec<ListingCandidate>, Error> {
    isin::validate(isin).map_err(|e| Error::Parse(e.to_string()))?;

    let mut candidates = Vec::new();
    for (i, chunk) in mics.chunks(MAX_JOBS_PER_REQUEST).enumerate() {
        if i > 0 {
            sleep(DELAY_BETWEEN_CHUNKS).await;
        }
        candidates.extend(fetch_chunk(client, isin, chunk).await?);
    }

    Ok(candidates)
}

async fn fetch_chunk(
    client: &Client,
    isin: &str,
    mics: &[&str],
) -> Result<Vec<ListingCandidate>, Error> {
    let payload: Vec<FigiRequest> = mics
        .iter()
        .map(|mic| FigiRequest {
            id_type: "ID_ISIN",
            id_value: isin,
            mic_code: mic,
        })
        .collect();

    let response = client
        .post("https://api.openfigi.com/v3/mapping")
        .json(&payload)
        .send()
        .await?;
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

    let results: Vec<FigiResponse> =
        serde_json::from_slice(&bytes).map_err(|e| Error::Parse(e.to_string()))?;

    if results.len() != mics.len() {
        return Err(Error::Parse(format!(
            "Expected {} results for {} requested MICs, got {}",
            mics.len(),
            mics.len(),
            results.len()
        )));
    }

    let candidates = mics
        .iter()
        .zip(results)
        .filter_map(|(&mic, res)| {
            // can have multiple listings per MIC, the first one is usually the primary one
            let hit = res.data?.into_iter().next()?;
            build_candidate(mic, hit)
        })
        .collect();

    Ok(candidates)
}

fn build_candidate(mic: &str, data: FigiData) -> Option<ListingCandidate> {
    Some(ListingCandidate {
        mic: mic.to_string(),
        ticker: data.ticker?,
        name: data.name.unwrap_or_default(),
        security_type: data.security_type,
        security_type_2: data.security_type_2,
        market_sector: data.market_sector,
    })
}

pub fn guess_instrument_type(candidate: &ListingCandidate) -> Option<InstrumentType> {
    // Catch ETFs first (Bloomberg puts them in the "Equity" sector)
    if candidate.security_type.as_deref() == Some("ETP") {
        return Some(InstrumentType::Etf);
    }

    match candidate.market_sector.as_deref() {
        Some("Equity" | "Pfd") => Some(InstrumentType::Stock),
        Some("Govt" | "Corp" | "Mtge" | "Muni") => Some(InstrumentType::Bond),
        _ => None,
    }
}

pub fn guess_accumulating(name: &str) -> Option<bool> {
    let name = name.to_uppercase();
    let words: Vec<_> = name.split(|c: char| !c.is_alphanumeric()).collect();

    let is_acc = words
        .iter()
        .any(|&w| matches!(w, "ACC" | "AC" | "ACCUMULATING"));
    let is_dist = words
        .iter()
        .any(|&w| matches!(w, "DIS" | "DIST" | "DISTRIBUTING"));

    match (is_acc, is_dist) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    }
}
