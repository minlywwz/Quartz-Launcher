use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::credentials::{get_secret, set_secret};

const SERVICE: &str = "quartz-launcher";

#[derive(Debug, Error)]
pub enum XboxError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Xbox auth failed: {0}")]
    Auth(String),
    #[error("Minecraft profile missing — account may not own Java Edition")]
    NoProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftSession {
    pub username: String,
    pub uuid: Uuid,
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

pub async fn authenticate_minecraft(msa_access_token: &str) -> Result<MinecraftSession, XboxError> {
    let xbl = xbox_authenticate(msa_access_token).await?;
    let xsts = xsts_authenticate(&xbl.token).await?;
    let mc_token = minecraft_login(&xbl.uhs, &xsts.token).await?;
    let profile = fetch_minecraft_profile(&mc_token.access_token).await?;

    Ok(MinecraftSession {
        username: profile.name,
        uuid: profile.id,
        access_token: mc_token.access_token,
        refresh_token: None,
    })
}

pub fn store_session(session: &MinecraftSession) -> keyring::Result<()> {
    let payload = serde_json::to_string(session).expect("session serializes");
    set_secret(SERVICE, &session.uuid.to_string(), &payload)
}

pub fn load_session(uuid: &Uuid) -> keyring::Result<Option<MinecraftSession>> {
    match get_secret(SERVICE, &uuid.to_string()) {
        Ok(raw) => Ok(Some(serde_json::from_str(&raw).unwrap_or_else(|_| {
            MinecraftSession {
                username: String::new(),
                uuid: *uuid,
                access_token: raw,
                refresh_token: None,
            }
        }))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e),
    }
}

struct XboxAuth {
    token: String,
    uhs: String,
}

struct XstsAuth {
    token: String,
}

struct McToken {
    access_token: String,
}

struct McProfile {
    id: Uuid,
    name: String,
}

async fn xbox_authenticate(msa_token: &str) -> Result<XboxAuth, XboxError> {
    let body = serde_json::json!({
        "Properties": {
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": format!("d={msa_token}")
        },
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    });

    let resp: XboxResponse = reqwest::Client::new()
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    let uhs = resp
        .display_claims
        .xui
        .first()
        .and_then(|x| x.uhs.clone())
        .ok_or_else(|| XboxError::Auth("missing Xbox uhs".into()))?;

    Ok(XboxAuth {
        token: resp.token,
        uhs,
    })
}

async fn xsts_authenticate(xbox_token: &str) -> Result<XstsAuth, XboxError> {
    let body = serde_json::json!({
        "Properties": {
            "SandboxId": "RETAIL",
            "UserTokens": [xbox_token]
        },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });

    let resp: XboxResponse = reqwest::Client::new()
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    Ok(XstsAuth { token: resp.token })
}

async fn minecraft_login(uhs: &str, xsts_token: &str) -> Result<McToken, XboxError> {
    let body = serde_json::json!({
        "identityToken": format!("XBL3.0 x={uhs};{xsts_token}")
    });

    let resp: McTokenResponse = reqwest::Client::new()
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    Ok(McToken {
        access_token: resp.access_token,
    })
}

async fn fetch_minecraft_profile(access_token: &str) -> Result<McProfile, XboxError> {
    let resp = reqwest::Client::new()
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(access_token)
        .send()
        .await?;

    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(XboxError::NoProfile);
    }

    if !resp.status().is_success() {
        return Err(XboxError::Auth(format!("profile HTTP {}", resp.status())));
    }

    let profile: McProfileResponse = resp.json().await?;
    let id = parse_minecraft_uuid(&profile.id)?;

    Ok(McProfile {
        id,
        name: profile.name,
    })
}

#[derive(Debug, Deserialize)]
struct XboxResponse {
    token: String,
    #[serde(default)]
    display_claims: DisplayClaims,
}

#[derive(Debug, Default, Deserialize)]
struct DisplayClaims {
    #[serde(default)]
    xui: Vec<XuiClaim>,
}

#[derive(Debug, Deserialize)]
struct XuiClaim {
    #[serde(default)]
    uhs: Option<String>,
}

#[derive(Debug, Deserialize)]
struct McTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct McProfileResponse {
    id: String,
    name: String,
}

fn parse_minecraft_uuid(raw: &str) -> Result<Uuid, XboxError> {
    if raw.contains('-') {
        return Uuid::parse_str(raw).map_err(|e| XboxError::Auth(e.to_string()));
    }
    if raw.len() != 32 {
        return Err(XboxError::Auth(format!("invalid profile uuid: {raw}")));
    }
    let formatted = format!(
        "{}-{}-{}-{}-{}",
        &raw[0..8],
        &raw[8..12],
        &raw[12..16],
        &raw[16..20],
        &raw[20..32]
    );
    Uuid::parse_str(&formatted).map_err(|e| XboxError::Auth(e.to_string()))
}
