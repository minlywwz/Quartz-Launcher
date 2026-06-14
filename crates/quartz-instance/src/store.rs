use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;
use uuid::Uuid;

use crate::Instance;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("instance not found: {0}")]
    NotFound(Uuid),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct InstanceStore {
    root: PathBuf,
}

impl InstanceStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_root() -> Option<PathBuf> {
        dirs::data_dir().map(|d| d.join("Quartz").join("instances"))
    }

    fn path_for(&self, id: Uuid) -> PathBuf {
        self.root.join(format!("{id}.json"))
    }

    pub fn create(&self, instance: &Instance) -> Result<(), StoreError> {
        fs::create_dir_all(&self.root)?;
        self.save(instance)
    }

    pub fn read(&self, id: Uuid) -> Result<Instance, StoreError> {
        let path = self.path_for(id);
        if !path.exists() {
            return Err(StoreError::NotFound(id));
        }
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn save(&self, instance: &Instance) -> Result<(), StoreError> {
        fs::create_dir_all(&self.root)?;
        let json = serde_json::to_string_pretty(instance)?;
        fs::write(self.path_for(instance.id), json)?;
        Ok(())
    }

    pub fn delete(&self, id: Uuid) -> Result<(), StoreError> {
        let path = self.path_for(id);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<Instance>, StoreError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }

        let mut instances = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(instance) = Self::read_file(&path) {
                    instances.push(instance);
                }
            }
        }
        Ok(instances)
    }

    fn read_file(path: &Path) -> Result<Instance, StoreError> {
        let json = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&json)?)
    }
}
