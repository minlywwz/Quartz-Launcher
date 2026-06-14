use std::path::{Path, PathBuf};

use serde_json::Value;
use thiserror::Error;

use crate::install::InstalledVersion;
use crate::loaders::LoaderInstallResult;
use crate::version::{is_native_artifact_path, library_allowed};

#[derive(Debug, Clone)]
pub struct LaunchProfile {
    pub main_class: String,
    pub classpath: Vec<PathBuf>,
    pub version_id: String,
    pub asset_index_id: String,
    pub jvm_args: Vec<String>,
}

#[derive(Debug, Error)]
pub enum LaunchProfileError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl LaunchProfile {
    pub fn from_vanilla(installed: &InstalledVersion) -> Self {
        Self {
            main_class: installed.main_class.clone(),
            classpath: installed.classpath.clone(),
            version_id: installed.id.clone(),
            asset_index_id: installed.asset_index_id.clone(),
            jvm_args: natives_jvm_args(&installed.natives_dir),
        }
    }

    pub fn from_loader(loader: &LoaderInstallResult, installed: &InstalledVersion) -> Self {
        let mut jvm_args = natives_jvm_args(&installed.natives_dir);
        jvm_args.extend(loader.jvm_args.clone());
        Self {
            main_class: loader.main_class.clone(),
            classpath: loader.classpath.clone(),
            version_id: loader.version_id.clone(),
            asset_index_id: installed.asset_index_id.clone(),
            jvm_args,
        }
    }

    pub async fn merge_profile_json(
        mut self,
        game_dir: &Path,
        profile: Value,
    ) -> Result<Self, LaunchProfileError> {
        if let Some(main) = profile.get("mainClass").and_then(|v| v.as_str()) {
            self.main_class = main.to_owned();
        }
        if let Some(id) = profile.get("id").and_then(|v| v.as_str()) {
            self.version_id = id.to_owned();
        }
        if let Some(libraries) = profile.get("libraries").and_then(|v| v.as_array()) {
            for lib in libraries {
                if let Ok(lib) = serde_json::from_value::<crate::version::Library>(lib.clone()) {
                    if library_allowed(&lib.rules) {
                        if let Some(artifact) = lib.downloads.and_then(|d| d.artifact) {
                            if is_native_artifact_path(&artifact.path) {
                                continue;
                            }
                            let path = crate::version::library_artifact_path(game_dir, &artifact.path);
                            if path.exists() && !self.classpath.iter().any(|p| p == &path) {
                                self.classpath.push(path);
                            }
                        }
                    }
                }
            }
        }
        Ok(self)
    }
}

pub fn natives_jvm_args(natives_dir: &Path) -> Vec<String> {
    let natives = natives_dir.display().to_string();
    vec![
        format!("-Djava.library.path={natives}"),
        format!("-Djna.tmpdir={natives}"),
        format!("-Dorg.lwjgl.system.SharedLibraryExtractPath={natives}"),
        format!("-Dio.netty.native.workdir={natives}"),
    ]
}

pub fn jvm_args_from_profile(profile: &Value) -> Vec<String> {
    let Some(jvm) = profile
        .get("arguments")
        .and_then(|a| a.get("jvm"))
        .and_then(|v| v.as_array())
    else {
        return Vec::new();
    };

    let mut args = Vec::new();
    for entry in jvm {
        if let Some(s) = entry.as_str() {
            args.push(s.to_owned());
        } else if let Some(values) = entry.get("value") {
            if let Some(s) = values.as_str() {
                args.push(s.to_owned());
            } else if let Some(list) = values.as_array() {
                for v in list {
                    if let Some(s) = v.as_str() {
                        args.push(s.to_owned());
                    }
                }
            }
        }
    }
    args
}
