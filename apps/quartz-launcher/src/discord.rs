use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use thiserror::Error;

static RPC: Lazy<Mutex<Option<DiscordRpc>>> = Lazy::new(|| Mutex::new(None));

const DEFAULT_APP_ID: &str = "1515508260914856047";

pub struct DiscordRpc {
    client: DiscordIpcClient,
    #[allow(dead_code)]
    app_id: String,
}

#[derive(Debug, Error)]
pub enum DiscordError {
    #[error("DISCORD_APP_ID environment variable is not set")]
    MissingAppId,
    #[error("Discord RPC error: {0}")]
    Client(String),
}

impl DiscordRpc {
    pub fn enable() -> Result<(), DiscordError> {
        if RPC.lock().is_some() {
            return Ok(());
        }

        let app_id =
            std::env::var("DISCORD_APP_ID").unwrap_or_else(|_| DEFAULT_APP_ID.to_string());
        let mut client = DiscordIpcClient::new(&app_id)
            .map_err(|e| DiscordError::Client(e.to_string()))?;
        client
            .connect()
            .map_err(|e| DiscordError::Client(e.to_string()))?;

        let payload = activity::Activity::new()
            .state("Browsing modpacks")
            .details("Quartz Launcher");
        client
            .set_activity(payload)
            .map_err(|e| DiscordError::Client(e.to_string()))?;

        *RPC.lock() = Some(Self { client, app_id });
        Ok(())
    }

    pub fn set_playing(modpack: &str, version: &str) -> Result<(), DiscordError> {
        let mut guard = RPC.lock();
        let rpc = guard
            .as_mut()
            .ok_or_else(|| DiscordError::Client("Discord RPC not enabled".into()))?;

        let state = format!("Playing {modpack}");
        let details = format!("Minecraft {version}");
        let payload = activity::Activity::new()
            .state(&state)
            .details(&details);
        rpc.client
            .set_activity(payload)
            .map_err(|e| DiscordError::Client(e.to_string()))?;
        Ok(())
    }

    pub fn set_browsing() -> Result<(), DiscordError> {
        let mut guard = RPC.lock();
        let rpc = guard
            .as_mut()
            .ok_or_else(|| DiscordError::Client("Discord RPC not enabled".into()))?;

        let payload = activity::Activity::new()
            .state("Browsing modpacks")
            .details("Quartz Launcher");
        rpc.client
            .set_activity(payload)
            .map_err(|e| DiscordError::Client(e.to_string()))?;
        Ok(())
    }

    pub fn clear() {
        if let Some(rpc) = RPC.lock().as_mut() {
            let _ = rpc.client.clear_activity();
            let _ = rpc.client.close();
        }
        *RPC.lock() = None;
    }
}

pub fn sync_enabled(enabled: bool) {
    if enabled {
        let _ = DiscordRpc::enable();
    } else {
        DiscordRpc::clear();
    }
}
