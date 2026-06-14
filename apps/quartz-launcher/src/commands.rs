use std::collections::HashMap;

use std::path::PathBuf;

use std::time::Duration;

use quartz_auth::{poll_device_code, start_device_code, Account};

use quartz_catalog::{CatalogBuildOptions, CatalogService, ModpackSource};

use quartz_instance::{Instance, InstanceStore};

use quartz_java::ensure_java_for_version;

use quartz_launch::{early_exit_message, LaunchArgsBuilder, spawn_process_with_log};

use quartz_meta::{

    ensure_loader_for_launch, fetch_version_manifest, install_version_by_id, install_with_loader,
    LaunchProfile, LoaderKind,

};

use quartz_modplatforms::{download_and_install_mrpack, CurseForgeClient, ModrinthClient};

use serde::{Deserialize, Serialize};

use serde_json::{Map, Value};

use tauri::{State, Window};

use uuid::Uuid;

use crate::discord::{sync_enabled, DiscordRpc};
use crate::game_sessions::InstanceRunState;
use crate::state::{AppState, PendingMsaLogin};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {

    pub success: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub running_count: Option<u32>,

}

#[derive(Debug, Serialize)]

#[serde(rename_all = "camelCase")]

pub struct AuthResult {

    pub success: bool,

    #[serde(skip_serializing_if = "Option::is_none")]

    pub username: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]

    pub uuid: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]

    pub user_code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]

    pub verification_uri: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_reauth: Option<bool>,

}

#[derive(Debug, Serialize)]

#[serde(rename_all = "camelCase")]

pub struct ModpackInfo {

    pub id: String,

    pub name: String,

    pub description: String,

    pub version: String,

    pub mc_version: String,

    pub loader: String,

    #[serde(skip_serializing_if = "Option::is_none")]

    pub icon_url: Option<String>,

    pub mod_count: u32,

    pub download_size: String,

    pub installed: bool,

}

#[derive(Debug, Serialize)]

#[serde(rename_all = "camelCase")]

pub struct InstanceInfo {

    pub id: String,

    pub name: String,

    pub minecraft_version: String,

    pub loader: String,

    #[serde(skip_serializing_if = "Option::is_none")]

    pub modpack_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]

    pub icon_url: Option<String>,

    pub memory_mb: u32,

}

#[derive(Debug, Deserialize)]

#[serde(rename_all = "camelCase")]

pub struct CreateInstanceRequest {

    pub name: String,

    pub minecraft_version: String,

    pub loader: String,

    #[serde(default)]

    pub modpack_id: Option<String>,

    #[serde(default)]

    pub icon_url: Option<String>,

}

fn instance_to_info(instance: &Instance) -> InstanceInfo {

    InstanceInfo {

        id: instance.id.to_string(),

        name: instance.name.clone(),

        minecraft_version: instance.minecraft_version.clone(),

        loader: instance

            .loader

            .clone()

            .unwrap_or_else(|| "vanilla".into()),

        modpack_id: instance.modpack_id.clone(),

        icon_url: instance.icon_url.clone(),

        memory_mb: instance.memory_mb,

    }

}

fn instance_store(state: &AppState) -> InstanceStore {

    InstanceStore::new(state.data_dir().join("instances"))

}

fn shared_game_dir(state: &AppState) -> PathBuf {

    state.data_dir().clone()

}

async fn resolve_java_for_mc(state: &AppState, mc_version: &str) -> Result<String, String> {
    if let Some(custom) = state.get_string_setting("javaPath").filter(|p| !p.is_empty()) {
        if std::path::Path::new(&custom).is_file() {
            return Ok(custom);
        }
    }

    ensure_java_for_version(mc_version, state.data_dir())
        .await
        .map(|p| p.to_string_lossy().into_owned())
        .or_else(|_| discover_system_java())
}

fn discover_system_java() -> Result<String, String> {
    if cfg!(windows) {
        let output = std::process::Command::new("where")
            .arg("java")
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let path = line.trim();
                if path.is_empty() {
                    continue;
                }
                if path.contains("jdk-21") || path.contains("jdk-22") || path.contains("jdk-23") {
                    return Ok(path.to_string());
                }
            }
            if let Some(first) = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
            {
                return Ok(first.to_string());
            }
        }
    }
    Err("java not on PATH".into())
}

fn catalog_options(state: &AppState) -> CatalogBuildOptions {
    CatalogBuildOptions {
        include_snapshots: state.get_bool_setting("showSnapshots"),
    }
}

async fn build_launch_profile(

    state: &AppState,

    instance: &Instance,

) -> Result<LaunchProfile, String> {

    let shared = shared_game_dir(state);

    let java = resolve_java_for_mc(state, &instance.minecraft_version).await?;

    let loader = instance

        .loader

        .as_deref()

        .map(LoaderKind::from_str)

        .unwrap_or(LoaderKind::Vanilla);

    let (installed, loader_result) = ensure_loader_for_launch(

        &instance.minecraft_version,

        loader,

        &shared,

        &java,

    )

    .await

    .map_err(|e| e.to_string())?;

    if loader == LoaderKind::Vanilla {

        return Ok(LaunchProfile::from_vanilla(&installed));

    }

    Ok(LaunchProfile::from_loader(
        &loader_result,
        &installed,
    ))

}

#[tauri::command]

pub fn get_settings(state: State<'_, AppState>) -> HashMap<String, Value> {

    state.settings.read().clone()

}

#[tauri::command]

pub fn save_settings(

    state: State<'_, AppState>,

    settings: HashMap<String, Value>,

) -> Result<(), String> {

    let rpc_enabled = settings
        .get("enableDiscordRpc")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    *state.settings.write() = settings;

    state.sync_active_account_from_settings();

    state.save_settings_to_disk().map_err(|e| e.to_string())?;

    sync_enabled(rpc_enabled);

    Ok(())

}

#[tauri::command]

pub async fn catalog_tree(state: State<'_, AppState>) -> Result<quartz_catalog::CatalogTree, String> {

    CatalogService::new()

        .build_tree(&catalog_options(&state))

        .await

        .map_err(|e| e.to_string())

}

#[tauri::command]
pub fn get_system_memory_mb() -> Result<u32, String> {
    Ok(crate::system_memory::total_memory_mb())
}

#[tauri::command]

pub async fn get_mc_versions(state: State<'_, AppState>) -> Result<Vec<String>, String> {

    let manifest = fetch_version_manifest()

        .await

        .map_err(|e| e.to_string())?;

    let include_snapshots = state.get_bool_setting("showSnapshots");

    Ok(manifest

        .versions

        .into_iter()

        .filter(|v| {

            if v.kind == "snapshot" {

                return include_snapshots;

            }

            if v.kind != "release" {

                return false;

            }

            let pre = v.id.contains("-pre") || v.id.contains("-rc");

            if pre {

                return include_snapshots;

            }

            v.id.starts_with("1.")

        })

        .map(|v| v.id)

        .collect())

}

#[tauri::command]

pub fn list_instances(state: State<'_, AppState>) -> Result<Vec<InstanceInfo>, String> {

    let mut instances = instance_store(&state)

        .list()

        .map_err(|e| e.to_string())?;

    instances.sort_by(|a, b| {

        b.minecraft_version

            .cmp(&a.minecraft_version)

            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))

    });

    Ok(instances.iter().map(instance_to_info).collect())

}

#[tauri::command]

pub async fn create_instance(

    state: State<'_, AppState>,

    request: CreateInstanceRequest,

) -> Result<InstanceInfo, String> {

    let name = request.name.trim();

    if name.is_empty() {

        return Err("instance name is required".into());

    }

    let instance = create_instance_internal(

        &state,

        name.to_owned(),

        request.minecraft_version,

        request.loader,

        request.modpack_id,

        request.icon_url,

        None,

        None,

    )

    .await?;

    Ok(instance_to_info(&instance))

}

#[tauri::command]

pub async fn launch_instance(

    state: State<'_, AppState>,

    window: Window,

    instance_id: String,

) -> Result<LaunchResult, String> {

    let id = Uuid::parse_str(&instance_id).map_err(|e| e.to_string())?;

    let instance = instance_store(&state)

        .read(id)

        .map_err(|e| e.to_string())?;

    launch_instance_record(&state, &window, &instance).await

}

#[tauri::command]

pub fn delete_instance(state: State<'_, AppState>, instance_id: String) -> Result<(), String> {

    let id = Uuid::parse_str(&instance_id).map_err(|e| e.to_string())?;

    instance_store(&state)

        .delete(id)

        .map_err(|e| e.to_string())

}

#[tauri::command]

pub async fn search_modpacks(

    mc_version: String,

    query: Option<String>,

) -> Result<Vec<ModpackInfo>, String> {

    let query = query.unwrap_or_default();

    let service = CatalogService::new();

    let entries = service

        .search_modpacks(&mc_version, &query, 20)

        .await

        .map_err(|e| e.to_string())?;

    Ok(entries

        .into_iter()

        .map(|e| ModpackInfo {

            id: e.id,

            name: e.name,

            description: e.description.unwrap_or_default(),

            version: e.version,

            mc_version: e.minecraft_version,

            loader: e.loader.unwrap_or_else(|| "unknown".into()),

            icon_url: e.icon_url,

            mod_count: 0,

            download_size: "unknown".into(),

            installed: false,

        })

        .collect())

}

#[tauri::command]

pub async fn list_modpacks(

    state: State<'_, AppState>,

    mc_version: Option<String>,

) -> Result<Vec<ModpackInfo>, String> {

    let store = instance_store(&state);

    let installed: Vec<_> = store.list().unwrap_or_default();

    let tree = CatalogService::new()

        .build_tree(&catalog_options(&state))

        .await

        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();

    for category in tree.categories {

        if let Some(ref ver) = mc_version {

            if category.id != *ver {

                continue;

            }

        }

        for entry in category.modpacks {

            let installed_match = installed.iter().any(|i| {

                i.modpack_id

                    .as_deref()

                    .map(|id| id == entry.id)

                    .unwrap_or(false)

            });

            results.push(ModpackInfo {

                id: entry.id.clone(),

                name: entry.name,

                description: entry.description.unwrap_or_default(),

                version: entry.version,

                mc_version: entry.minecraft_version,

                loader: entry

                    .loader

                    .unwrap_or_else(|| match entry.source {

                        ModpackSource::Vanilla => "vanilla".into(),

                        _ => "fabric".into(),

                    }),

                icon_url: entry.icon_url,

                mod_count: 0,

                download_size: "unknown".into(),

                installed: installed_match,

            });

        }

    }

    Ok(results)

}

#[tauri::command]

pub async fn apply_default_preset(state: State<'_, AppState>) -> Result<LaunchResult, String> {

    let preset = state

        .get_string_setting("defaultPreset")

        .unwrap_or_else(|| "latest-vanilla".into());

    let manifest = fetch_version_manifest()

        .await

        .map_err(|e| e.to_string())?;

    let mc_version = manifest

        .versions

        .iter()

        .find(|v| {

            v.kind == "release"

                && v.id.starts_with("1.")

                && !v.id.contains("-pre")

                && !v.id.contains("-rc")

        })

        .map(|v| v.id.clone())

        .unwrap_or_else(|| manifest.latest.release.clone());

    match preset.as_str() {

        "latest-fabric-optimized" => {

            let instance = create_instance_internal(

                &state,

                "Fabulously Optimized".into(),

                mc_version.clone(),

                "fabric".into(),

                Some("modrinth:fabulously-optimized".into()),

                None,

                Some(mc_version),

                Some("fabric".into()),

            )

            .await?;

            Ok(LaunchResult {

                success: true,

                message: Some(format!("Created {}", instance.name)),

                session_id: None,

                running_count: None,

            })

        }

        _ => {

            let instance = create_instance_internal(

                &state,

                format!("Vanilla {mc_version}"),

                mc_version.clone(),

                "vanilla".into(),

                Some(format!("vanilla:{mc_version}")),

                None,

                None,

                None,

            )

            .await?;

            Ok(LaunchResult {

                success: true,

                message: Some(format!("Created {}", instance.name)),

                session_id: None,

                running_count: None,

            })

        }

    }

}

async fn create_instance_internal(

    state: &AppState,

    name: String,

    mc_version: String,

    loader: String,

    modpack_id: Option<String>,

    icon_url: Option<String>,

    mc_override: Option<String>,

    loader_override: Option<String>,

) -> Result<Instance, String> {

    let shared = shared_game_dir(state);

    if let Some(pack_id) = modpack_id.clone() {

        let parts: Vec<_> = pack_id.split(':').collect();

        if parts.len() == 2 && parts[0] == "modrinth" {

            let slug = parts[1];

            let client = ModrinthClient::default();

            let versions = client

                .get_project_versions(slug)

                .await

                .map_err(|e| e.to_string())?;

            let version = versions

                .first()

                .ok_or_else(|| "no versions found".to_string())?;

            let url = client

                .get_version_download_url(&version.id)

                .await

                .map_err(|e| e.to_string())?;

            let mc_version = mc_override

                .or_else(|| version.game_versions.first().cloned())

                .unwrap_or(mc_version);

            let loader = loader_override

                .or_else(|| version.loaders.first().cloned())

                .unwrap_or(loader);

            let loader_kind = LoaderKind::from_str(&loader);

            let java = resolve_java_for_mc(state, &mc_version).await?;

            install_with_loader(&mc_version, loader_kind, &shared, &java)

                .await

                .map_err(|e| e.to_string())?;

            let mut instance = Instance::new(name, &mc_version);

            instance.memory_mb = state.memory_mb();

            instance.modpack_id = Some(pack_id.clone());

            instance.loader = Some(loader);

            instance.icon_url = icon_url;

            let instance_dir = state

                .data_dir()

                .join("instances")

                .join(instance.id.to_string());

            instance.game_dir = Some(instance_dir.clone());

            instance_store(state)

                .create(&instance)

                .map_err(|e| e.to_string())?;

            let mrpack_path = state

                .data_dir()

                .join("modpacks")

                .join(slug)

                .join(&version.files[0].filename);

            download_and_install_mrpack(&url, &mrpack_path, &instance_dir)

                .await

                .map_err(|e| e.to_string())?;

            return Ok(instance);

        }

        if parts.len() == 2 && parts[0] == "vanilla" {

            let mc_version = mc_override.unwrap_or_else(|| parts[1].to_string());

            install_version_by_id(&mc_version, &shared)

                .await

                .map_err(|e| e.to_string())?;

            let mut instance = Instance::new(name, &mc_version);

            instance.memory_mb = state.memory_mb();

            instance.modpack_id = Some(pack_id.clone());

            instance.loader = Some("vanilla".into());

            instance.icon_url = icon_url;

            let instance_dir = state

                .data_dir()

                .join("instances")

                .join(instance.id.to_string());

            instance.game_dir = Some(instance_dir);

            instance_store(state)

                .create(&instance)

                .map_err(|e| e.to_string())?;

            return Ok(instance);

        }

    }

    let mc_version = mc_override.unwrap_or(mc_version);

    let loader = loader_override.unwrap_or(loader);

    let loader_kind = LoaderKind::from_str(&loader);

    if loader_kind == LoaderKind::Vanilla {

        install_version_by_id(&mc_version, &shared)

            .await

            .map_err(|e| e.to_string())?;

    } else {

        let java = resolve_java_for_mc(state, &mc_version).await?;

        install_with_loader(&mc_version, loader_kind, &shared, &java)

            .await

            .map_err(|e| e.to_string())?;

    }

    let mut instance = Instance::new(name, &mc_version);

    instance.memory_mb = state.memory_mb();

    instance.loader = Some(loader);

    instance.icon_url = icon_url;

    let instance_dir = state

        .data_dir()

        .join("instances")

        .join(instance.id.to_string());

    instance.game_dir = Some(instance_dir);

    instance_store(state)

        .create(&instance)

        .map_err(|e| e.to_string())?;

    Ok(instance)

}

async fn download_modpack_internal(

    state: &AppState,

    pack_id: String,

    mc_override: Option<String>,

    loader_override: Option<String>,

) -> Result<LaunchResult, String> {

    let parts: Vec<_> = pack_id.split(':').collect();

    if parts.len() != 2 {

        return Ok(LaunchResult {

            success: false,

            message: Some("invalid pack id format".into()),

            session_id: None,

            running_count: None,

        });

    }

    let default_name = match parts[0] {

        "modrinth" => parts[1].to_string(),

        "vanilla" => format!("Vanilla {}", parts[1]),

        other => format!("{other} {}", parts[1]),

    };

    let loader = loader_override

        .clone()

        .unwrap_or_else(|| if parts[0] == "vanilla" { "vanilla".into() } else { "fabric".into() });

    let mc_version = mc_override.clone().unwrap_or_else(|| {

        if parts[0] == "vanilla" {

            parts[1].to_string()

        } else {

            "1.21.1".into()

        }

    });

    let instance = create_instance_internal(

        state,

        default_name,

        mc_version.clone(),

        loader,

        Some(pack_id),

        None,

        mc_override,

        loader_override,

    )

    .await?;

    Ok(LaunchResult {

        success: true,

        message: Some(format!("Created {}", instance.name)),

        session_id: None,

        running_count: None,

    })

}

async fn launch_instance_record(

    state: &AppState,

    window: &Window,

    instance: &Instance,

) -> Result<LaunchResult, String> {

    let account = state

        .active_account_or_restore()

        .ok_or_else(|| "no active account — log in first".to_string())?;

    let profile = build_launch_profile(state, instance).await?;

    let game_dir = instance.resolved_game_dir(&state.data_dir().join("instances"));

    tokio::fs::create_dir_all(&game_dir)

        .await

        .map_err(|e| e.to_string())?;

    let java = resolve_java_for_mc(state, &instance.minecraft_version).await?;

    let memory = state.memory_mb();

    let command = LaunchArgsBuilder::new(java)
        .game_dir(&game_dir)
        .classpath(profile.classpath.clone())
        .jvm_arg(format!("-Xmx{}M", memory))
        .for_instance(instance, &account, &shared_game_dir(state), &profile)

        .build()

        .map_err(|e| e.to_string())?;

    let mut child = spawn_process_with_log(&command, Some(&game_dir), Some(&game_dir))
        .await
        .map_err(|e| e.to_string())?;

    if let Some(err) = early_exit_message(&mut child, Some(&game_dir), Duration::from_secs(3)).await {

        return Err(err);

    }

    if state.discord_enabled() {
        let _ = DiscordRpc::set_playing(&instance.name, &instance.minecraft_version);
    }

    let session = state.game_sessions.register(
        instance.id.to_string(),
        &instance.name,
        child,
    );
    let running_count = state
        .game_sessions
        .instance_state(&instance.id.to_string())
        .session_count as u32;

    state.refresh_settings_from_disk();
    if state.get_bool_setting("closeOnLaunch") {
        let _ = window.hide();
    }

    Ok(LaunchResult {
        success: true,
        message: Some("Game launched".into()),
        session_id: Some(session.session_id),
        running_count: Some(running_count),
    })

}

#[tauri::command]
pub fn get_instance_run_state(
    state: State<'_, AppState>,
    instance_id: String,
) -> InstanceRunState {
    state.game_sessions.instance_state(&instance_id)
}

#[tauri::command]
pub async fn stop_instance_game(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<u32, String> {
    let stopped = state
        .game_sessions
        .stop_all_for_instance(&instance_id)
        .await?;
    if state.discord_enabled() && state.game_sessions.total_running() == 0 {
        let _ = DiscordRpc::set_browsing();
    }
    Ok(stopped)
}

#[tauri::command]

pub async fn download_modpack(

    state: State<'_, AppState>,

    pack_id: String,

) -> Result<LaunchResult, String> {

    download_modpack_internal(&state, pack_id, None, None).await

}

#[tauri::command]

pub async fn launch_modpack(

    state: State<'_, AppState>,

    window: Window,

    pack_id: String,

) -> Result<LaunchResult, String> {

    let store = instance_store(&state);

    let instances = store.list().map_err(|e| e.to_string())?;

    let instance = instances

        .into_iter()

        .find(|i| i.modpack_id.as_deref() == Some(pack_id.as_str()))

        .ok_or_else(|| "modpack not installed — download first".to_string())?;

    launch_instance_record(&state, &window, &instance).await

}

#[tauri::command]

pub fn login_offline(state: State<'_, AppState>, username: String) -> AuthResult {

    let account = Account::offline(&username);
    let account_id = state
        .register_account_login(&account, false)
        .ok();

    AuthResult {

        success: true,

        username: Some(account.username().to_owned()),

        uuid: Some(account.uuid().hyphenated().to_string()),

        user_code: None,

        verification_uri: None,

        account_id,

        needs_reauth: None,

    }

}

#[tauri::command]

pub async fn login_microsoft(state: State<'_, AppState>) -> Result<AuthResult, String> {

    match start_device_code().await {

        Ok(info) => {

            *state.pending_msa.write() = Some(PendingMsaLogin {

                user_code: info.user_code.clone(),

                verification_uri: info.verification_uri.clone(),

                device_code: info.device_code.clone(),

            });

            Ok(AuthResult {

                success: true,

                username: None,

                uuid: None,

                user_code: Some(info.user_code),

                verification_uri: Some(info.verification_uri),

                account_id: None,

                needs_reauth: None,

            })

        }

        Err(_e) => Ok(AuthResult {

            success: false,

            username: None,

            uuid: None,

            user_code: None,

            verification_uri: None,

            account_id: None,

            needs_reauth: None,

        }),

    }

}

#[tauri::command]

pub async fn login_microsoft_poll(state: State<'_, AppState>) -> Result<AuthResult, String> {

    let device_code = state

        .pending_msa

        .read()

        .as_ref()

        .map(|p| p.device_code.clone());

    let Some(device_code) = device_code else {

        return Ok(AuthResult {

            success: false,

            username: None,

            uuid: None,

            user_code: None,

            verification_uri: None,

            account_id: None,

            needs_reauth: None,

        });

    };

    match poll_device_code(&device_code).await {

        Ok(account) => {

            let account_id = state
                .register_account_login(&account, true)
                .ok();

            *state.pending_msa.write() = None;

            Ok(AuthResult {

                success: true,

                username: Some(account.username().to_owned()),

                uuid: Some(account.uuid().hyphenated().to_string()),

                user_code: None,

                verification_uri: None,

                account_id,

                needs_reauth: None,

            })

        }

        Err(quartz_auth::MsaError::AuthorizationPending) => Ok(AuthResult {

            success: false,

            username: None,

            uuid: None,

            user_code: None,

            verification_uri: None,

            account_id: None,

            needs_reauth: None,

        }),

        Err(_e) => Ok(AuthResult {

            success: false,

            username: None,

            uuid: None,

            user_code: None,

            verification_uri: None,

            account_id: None,

            needs_reauth: None,

        }),

    }

}

#[tauri::command]
pub fn switch_account(state: State<'_, AppState>, account_id: String) -> AuthResult {
    match state.switch_saved_account(&account_id) {
        Ok(active_id) => {
            let saved = state
                .read_saved_accounts()
                .into_iter()
                .find(|a| a.id == active_id);
            AuthResult {
                success: true,
                username: saved.as_ref().map(|a| a.username.clone()),
                uuid: saved
                    .as_ref()
                    .and_then(|a| a.uuid.clone()),
                user_code: None,
                verification_uri: None,
                account_id: Some(active_id),
                needs_reauth: None,
            }
        }
        Err(message) if message == "needs_reauth" => AuthResult {
            success: false,
            username: None,
            uuid: None,
            user_code: None,
            verification_uri: None,
            account_id: Some(account_id),
            needs_reauth: Some(true),
        },
        Err(_message) => AuthResult {
            success: false,
            username: None,
            uuid: None,
            user_code: None,
            verification_uri: None,
            account_id: None,
            needs_reauth: None,
        },
    }
}

#[tauri::command]
pub fn sign_out_account(state: State<'_, AppState>) -> Result<(), String> {
    state.sign_out_current_account().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pick_java_path() -> Result<Option<String>, String> {
    let picked = rfd::FileDialog::new()
        .set_title("Select Java executable")
        .pick_file();
    Ok(picked.map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]

pub fn link_discord() -> Result<Map<String, Value>, String> {

    sync_enabled(true);

    let mut map = Map::new();

    map.insert("success".into(), Value::Bool(true));

    Ok(map)

}

#[tauri::command]

pub async fn minimize_window(window: Window) -> Result<(), String> {

    window.minimize().map_err(|e| e.to_string())

}

#[tauri::command]

pub async fn maximize_window(window: Window) -> Result<(), String> {

    if window.is_maximized().map_err(|e| e.to_string())? {

        window.unmaximize().map_err(|e| e.to_string())

    } else {

        window.maximize().map_err(|e| e.to_string())

    }

}

#[tauri::command]

pub async fn close_window(window: Window) -> Result<(), String> {

    window.close().map_err(|e| e.to_string())

}

#[tauri::command]

pub async fn open_external(url: String) -> Result<(), String> {

    open::that(&url).map_err(|e| e.to_string())

}

#[tauri::command]

pub async fn search_curseforge_modpacks(

    mc_version: String,

    query: String,

) -> Result<Vec<quartz_modplatforms::CurseForgeModpackHit>, String> {

    let client = CurseForgeClient::from_env().map_err(|e| e.to_string())?;

    client

        .search_modpacks(&mc_version, &query)

        .await

        .map_err(|e| e.to_string())

}

pub fn invoke_handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {

    tauri::generate_handler![

        get_settings,

        save_settings,

        list_modpacks,

        launch_modpack,

        download_modpack,

        apply_default_preset,

        login_offline,

        login_microsoft,

        login_microsoft_poll,

        switch_account,

        sign_out_account,

        link_discord,

        minimize_window,

        maximize_window,

        close_window,

        open_external,

        catalog_tree,

        get_mc_versions,

        get_system_memory_mb,

        list_instances,

        create_instance,

        launch_instance,

        get_instance_run_state,

        stop_instance_game,

        delete_instance,

        search_modpacks,

        search_curseforge_modpacks,

        pick_java_path,

    ]

}
