use crate::isin;
use reqwest::Client;
use serde::Serialize;
use specta::Type;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Response parse error: {0}")]
    Parse(String),

    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LeiInfo {
    pub domicile: String,
    pub legal_name: String,
}

pub async fn search_gleif(client: &Client, isin: &str) -> Result<LeiInfo, Error> {
    isin::validate(isin).map_err(|e| Error::Parse(e.to_string()))?;

    let response = client
        .get(format!(
            "https://api.gleif.org/api/v1/lei-records?filter[isin]={isin}"
        ))
        .send()
        .await?;
    let status = response.status();
    let bytes = response.bytes().await?;

    if !status.is_success() {
        let err_msg = if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            json.pointer("/message")
                .and_then(|m| m.as_str())
                .unwrap_or_else(|| "Unknown API error")
                .to_string()
        } else {
            String::from_utf8_lossy(&bytes).into_owned()
        };

        return Err(Error::Api {
            status: status.as_u16(),
            message: err_msg,
        });
    }

    let json: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| Error::Parse(e.to_string()))?;

    let legal_name = json
        .pointer("/data/0/attributes/entity/legalName/name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Parse(format!("Could not find legal name for ISIN {isin}")))?;

    let domicile = json
        .pointer("/data/0/attributes/entity/legalAddress/country")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Parse(format!("Could not find domicile for ISIN {isin}")))?;

    Ok(LeiInfo {
        domicile,
        legal_name,
    })
}
