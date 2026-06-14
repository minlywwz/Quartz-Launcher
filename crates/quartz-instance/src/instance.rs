use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: Uuid,
    pub name: String,
    pub minecraft_version: String,
    #[serde(default)]
    pub modloader: Option<String>,
    #[serde(default)]
    pub loader: Option<String>,
    #[serde(default)]
    pub modpack_id: Option<String>,
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u32,
    #[serde(default)]
    pub game_dir: Option<PathBuf>,

    #[serde(default)]
    pub icon_url: Option<String>,
}

fn default_memory_mb() -> u32 {
    4096
}

impl Instance {
    pub fn new(name: impl Into<String>, minecraft_version: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            minecraft_version: minecraft_version.into(),
            modloader: None,
            loader: None,
            modpack_id: None,
            memory_mb: default_memory_mb(),
            game_dir: None,
            icon_url: None,
        }
    }

    pub fn resolved_game_dir(&self, default_root: &std::path::Path) -> PathBuf {
        self.game_dir
            .clone()
            .unwrap_or_else(|| default_root.join(self.id.to_string()))
    }
}
