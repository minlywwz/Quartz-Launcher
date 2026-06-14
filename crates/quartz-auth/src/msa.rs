use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Account;

pub const MSA_DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
pub const MSA_TOKEN_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
pub const MSA_SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeStart {
    pub user_code: String,
    pub verification_uri: String,
    pub device_code: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum MsaError {
    #[error("AZURE_CLIENT_ID environment variable is not set")]
    MissingClientId,
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("device code flow failed: {0}")]
    DeviceCode(String),
    #[error("authentication pending — user has not completed sign-in yet")]
    AuthorizationPending,
    #[error("device code expired — start a new login")]
    ExpiredToken,
    #[error("failed to parse response: {0}")]
    Json(#[from] serde_json::Error),
    #[error("minecraft authentication failed: {0}")]
    Xbox(#[from] crate::xbox::XboxError),
}

fn client_id() -> Result<String, MsaError> {
    std::env::var("AZURE_CLIENT_ID").map_err(|_| MsaError::MissingClientId)
}

pub async fn start_device_code() -> Result<DeviceCodeStart, MsaError> {
    let client_id = client_id()?;
    let params = [
        ("client_id", client_id.as_str()),
        ("scope", MSA_SCOPE),
    ];

    let response = reqwest::Client::new()
        .post(MSA_DEVICE_CODE_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(MsaError::DeviceCode(body));
    }

    let body: DeviceCodeResponse = response.json().await?;
    Ok(DeviceCodeStart {
        user_code: body.user_code,
        verification_uri: body.verification_uri,
        device_code: body.device_code,
        expires_in: body.expires_in,
        interval: body.interval,
        message: body.message,
    })
}

pub async fn poll_device_code(device_code: &str) -> Result<Account, MsaError> {
    let client_id = client_id()?;
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("client_id", client_id.as_str()),
        ("device_code", device_code),
    ];

    let response = reqwest::Client::new()
        .post(MSA_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;

    let status = response.status();
    let body_text = response.text().await?;

    if status.is_success() {
        let token: TokenResponse = serde_json::from_str(&body_text)?;
        let session = crate::xbox::authenticate_minecraft(&token.access_token).await?;
        crate::xbox::store_session(&session).ok();
        return Ok(Account::Msa {
            username: session.username,
            uuid: session.uuid,
            access_token: session.access_token,
        });
    }

    let err: OAuthErrorResponse = serde_json::from_str(&body_text).unwrap_or(OAuthErrorResponse {
        error: "unknown".into(),
        error_description: Some(body_text),
    });

    match err.error.as_str() {
        "authorization_pending" => Err(MsaError::AuthorizationPending),
        "expired_token" => Err(MsaError::ExpiredToken),
        other => Err(MsaError::DeviceCode(
            err.error_description
                .unwrap_or_else(|| other.to_owned()),
        )),
    }
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    user_code: String,
    device_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
    message: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct OAuthErrorResponse {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}
