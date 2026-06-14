use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use parking_lot::RwLock;
use quartz_auth::Account;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingMsaLogin {
    pub user_code: String,
    pub verification_uri: String,
    pub device_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedAccountSettings {
    pub id: String,
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    pub microsoft_linked: bool,
}

use crate::game_sessions::GameSessions;

#[derive(Debug, Default)]
pub struct AppState {
    pub settings: RwLock<HashMap<String, Value>>,
    pub active_account: RwLock<Option<Account>>,
    pub pending_msa: RwLock<Option<PendingMsaLogin>>,
    pub game_sessions: GameSessions,
    settings_path: PathBuf,
    data_dir: PathBuf,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Quartz");
        fs::create_dir_all(&data_dir)?;

        let settings_path = quartz_settings_path();
        let settings = if settings_path.exists() {
            let raw = fs::read_to_string(&settings_path)?;
            serde_json::from_str(&raw).unwrap_or_default()
        } else {
            HashMap::new()
        };

        let state = Self {
            settings: RwLock::new(settings),
            active_account: RwLock::new(None),
            pending_msa: RwLock::new(None),
            game_sessions: GameSessions::default(),
            settings_path,
            data_dir,
        };
        state.migrate_account_settings();
        state.sync_active_account_from_settings();
        Ok(state)
    }

    fn migrate_account_settings(&self) {
        let mut settings = self.settings.write();
        if settings.get("accounts").is_some() {
            return;
        }

        let username = settings
            .get("offlineUsername")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let microsoft_linked = settings
            .get("microsoftLinked")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut accounts = Vec::<SavedAccountSettings>::new();
        let mut active_account_id = String::new();

        if let Some(username) = username {
            let uuid = Account::offline(username.to_owned())
                .uuid()
                .hyphenated()
                .to_string();
            let id = Uuid::new_v4().to_string();
            accounts.push(SavedAccountSettings {
                id: id.clone(),
                username: username.to_owned(),
                uuid: Some(uuid),
                microsoft_linked,
            });
            active_account_id = id;
        }

        settings.insert(
            "accounts".into(),
            serde_json::to_value(&accounts).unwrap_or(Value::Array(vec![])),
        );
        settings.insert(
            "activeAccountId".into(),
            Value::String(active_account_id),
        );
        drop(settings);
        let _ = self.save_settings_to_disk();
    }

    pub fn read_saved_accounts(&self) -> Vec<SavedAccountSettings> {
        self.settings
            .read()
            .get("accounts")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    pub fn active_account_id(&self) -> Option<String> {
        self.settings
            .read()
            .get("activeAccountId")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    }

    fn write_legacy_account_fields(settings: &mut HashMap<String, Value>, account: Option<&SavedAccountSettings>) {
        match account {
            Some(saved) => {
                settings.insert(
                    "offlineUsername".into(),
                    Value::String(saved.username.clone()),
                );
                settings.insert(
                    "microsoftLinked".into(),
                    Value::Bool(saved.microsoft_linked),
                );
            }
            None => {
                settings.insert("offlineUsername".into(), Value::String(String::new()));
                settings.insert("microsoftLinked".into(), Value::Bool(false));
            }
        }
    }

    pub fn register_account_login(
        &self,
        account: &Account,
        microsoft_linked: bool,
    ) -> anyhow::Result<String> {
        let username = account.username().to_owned();
        let uuid = account.uuid().hyphenated().to_string();
        let mut settings = self.settings.write();
        let mut accounts: Vec<SavedAccountSettings> = settings
            .get("accounts")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let existing_id = accounts
            .iter()
            .find(|a| {
                a.microsoft_linked == microsoft_linked
                    && a.username.eq_ignore_ascii_case(&username)
            })
            .map(|a| a.id.clone());

        let account_id = existing_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let saved = SavedAccountSettings {
            id: account_id.clone(),
            username: username.clone(),
            uuid: Some(uuid),
            microsoft_linked,
        };

        if let Some(entry) = accounts
            .iter_mut()
            .find(|a| a.id == account_id)
        {
            *entry = saved.clone();
        } else {
            accounts.push(saved.clone());
        }

        settings.insert(
            "accounts".into(),
            serde_json::to_value(&accounts)?,
        );
        settings.insert(
            "activeAccountId".into(),
            Value::String(account_id.clone()),
        );
        Self::write_legacy_account_fields(&mut settings, Some(&saved));
        drop(settings);

        *self.active_account.write() = Some(account.clone());
        self.save_settings_to_disk()?;
        Ok(account_id)
    }

    pub fn switch_saved_account(&self, account_id: &str) -> Result<String, String> {
        let accounts = self.read_saved_accounts();
        let Some(saved) = accounts.iter().find(|a| a.id == account_id) else {
            return Err("account not found".into());
        };

        if saved.microsoft_linked {
            let active = self.active_account.read();
            if let Some(Account::Msa { username, .. }) = active.as_ref() {
                if username.eq_ignore_ascii_case(&saved.username) {
                    let mut settings = self.settings.write();
                    settings.insert(
                        "activeAccountId".into(),
                        Value::String(saved.id.clone()),
                    );
                    Self::write_legacy_account_fields(&mut settings, Some(saved));
                    drop(settings);
                    let _ = self.save_settings_to_disk();
                    return Ok(saved.id.clone());
                }
            }
            return Err("needs_reauth".into());
        }

        let account = Account::offline(saved.username.clone());
        *self.active_account.write() = Some(account);

        let mut settings = self.settings.write();
        settings.insert(
            "activeAccountId".into(),
            Value::String(saved.id.clone()),
        );
        Self::write_legacy_account_fields(&mut settings, Some(saved));
        drop(settings);
        self.save_settings_to_disk()
            .map_err(|e| e.to_string())?;

        Ok(saved.id.clone())
    }

    pub fn sign_out_current_account(&self) -> anyhow::Result<()> {
        *self.active_account.write() = None;
        let mut settings = self.settings.write();
        settings.insert("activeAccountId".into(), Value::String(String::new()));
        Self::write_legacy_account_fields(&mut settings, None);
        self.save_settings_to_disk()
    }

    pub fn sync_active_account_from_settings(&self) {
        if self.active_account.read().is_some() {
            return;
        }

        let accounts = self.read_saved_accounts();
        let active_id = self.active_account_id();

        let saved = active_id
            .as_deref()
            .and_then(|id| accounts.iter().find(|a| a.id == id));

        if let Some(saved) = saved {
            if !saved.microsoft_linked {
                *self.active_account.write() =
                    Some(Account::offline(saved.username.clone()));
            }
            return;
        }

        let settings = self.settings.read();
        let username = settings
            .get("offlineUsername")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let microsoft_linked = settings
            .get("microsoftLinked")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if microsoft_linked {
            return;
        }

        if let Some(username) = username {
            *self.active_account.write() = Some(Account::offline(username.to_owned()));
        }
    }

    pub fn active_account_or_restore(&self) -> Option<Account> {
        if let Some(account) = self.active_account.read().clone() {
            return Some(account);
        }
        self.sync_active_account_from_settings();
        self.active_account.read().clone()
    }

    pub fn save_settings_to_disk(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.settings_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&*self.settings.read())?;
        fs::write(&self.settings_path, json)?;
        Ok(())
    }

    pub fn refresh_settings_from_disk(&self) {
        if !self.settings_path.is_file() {
            return;
        }
        let Ok(raw) = fs::read_to_string(&self.settings_path) else {
            return;
        };
        let Ok(parsed) = serde_json::from_str::<HashMap<String, Value>>(&raw) else {
            return;
        };
        *self.settings.write() = parsed;
        self.sync_active_account_from_settings();
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub fn get_string_setting(&self, key: &str) -> Option<String> {
        self.settings
            .read()
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    }

    pub fn get_bool_setting(&self, key: &str) -> bool {
        self.settings
            .read()
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub fn get_u32_setting(&self, key: &str) -> Option<u32> {
        self.settings.read().get(key).and_then(|v| {
            v.as_u64()
                .map(|n| n as u32)
                .or_else(|| v.as_i64().map(|n| n as u32))
        })
    }

    pub fn discord_enabled(&self) -> bool {
        self.settings
            .read()
            .get("enableDiscordRpc")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    }

    pub fn memory_mb(&self) -> u32 {
        let configured = self.get_u32_setting("memoryMb").unwrap_or(4096);
        let max = crate::system_memory::total_memory_mb();
        configured.clamp(1024, max)
    }
}

pub fn quartz_settings_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".quartz")
        .join("settings.json")
}
