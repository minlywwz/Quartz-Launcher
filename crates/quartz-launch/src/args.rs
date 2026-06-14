use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use quartz_auth::Account;
use quartz_instance::Instance;
use thiserror::Error;
use tokio::process::{Child, Command};

#[derive(Debug, Clone)]
pub struct LaunchCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
}

#[derive(Debug, Default)]
pub struct LaunchArgsBuilder {
    java: PathBuf,
    game_dir: PathBuf,
    classpath: Vec<PathBuf>,
    main_class: String,
    jvm_args: Vec<String>,
    game_args: Vec<String>,
    access_token: String,
}

impl LaunchArgsBuilder {
    pub fn new(java: impl Into<PathBuf>) -> Self {
        Self {
            java: java.into(),
            main_class: "net.minecraft.client.main.Main".into(),
            access_token: "0".into(),
            ..Self::default()
        }
    }

    pub fn game_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.game_dir = path.into();
        self
    }

    pub fn classpath(mut self, entries: Vec<PathBuf>) -> Self {
        self.classpath = entries;
        self
    }

    pub fn main_class(mut self, main_class: impl Into<String>) -> Self {
        self.main_class = main_class.into();
        self
    }

    pub fn access_token(mut self, token: impl Into<String>) -> Self {
        self.access_token = token.into();
        self
    }

    pub fn jvm_arg(mut self, arg: impl Into<String>) -> Self {
        self.jvm_args.push(arg.into());
        self
    }

    pub fn game_arg(mut self, arg: impl Into<String>) -> Self {
        self.game_args.push(arg.into());
        self
    }

    pub fn for_instance(
        mut self,
        _instance: &Instance,
        account: &Account,
        shared_dir: &std::path::Path,
        profile: &quartz_meta::LaunchProfile,
    ) -> Self {
        let game_dir = self.game_dir.display().to_string();
        let assets_dir = shared_dir.join("assets").display().to_string();
        let user_type = account.launch_user_type();

        self.access_token = account.access_token().to_owned();
        self.main_class = profile.main_class.clone();
        self.jvm_args.extend(profile.jvm_args.clone());
        let mut game_args = vec![
            "--username".to_owned(),
            account.username().to_owned(),
            "--version".to_owned(),
            profile.version_id.clone(),
            "--gameDir".to_owned(),
            game_dir,
            "--assetsDir".to_owned(),
            assets_dir,
            "--assetIndex".to_owned(),
            profile.asset_index_id.clone(),
            "--uuid".to_owned(),
            account.uuid().hyphenated().to_string(),
            "--accessToken".to_owned(),
            self.access_token.clone(),
            "--userType".to_owned(),
            user_type.to_owned(),
            "--versionType".to_owned(),
            "release".to_owned(),
        ];
        if account.kind() == quartz_auth::AccountKind::Offline {
            game_args.extend([
                "--userProperties".to_owned(),
                "{}".to_owned(),
            ]);
        }
        self.game_args.extend(game_args);
        self
    }

    pub fn build(self) -> Result<LaunchCommand, LaunchError> {
        if self.java.as_os_str().is_empty() {
            return Err(LaunchError::MissingJava);
        }

        let classpath = self
            .classpath
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(if cfg!(windows) { ";" } else { ":" });

        let mut args = self.jvm_args;
        if !classpath.is_empty() {
            args.push("-cp".into());
            args.push(classpath);
        }
        args.push(self.main_class);
        args.extend(self.game_args);

        let cmdline_len: usize = args.iter().map(String::len).sum::<usize>() + args.len();
        if cfg!(windows) && cmdline_len > 7_000 {
            let argfile = self.game_dir.join(".quartz-launch.args");
            write_argfile(&argfile, &args)?;
            Ok(LaunchCommand {
                program: self.java,
                args: vec![format!("@{}", argfile.display())],
            })
        } else {
            Ok(LaunchCommand {
                program: self.java,
                args,
            })
        }
    }
}

fn write_argfile(path: &Path, args: &[String]) -> Result<(), LaunchError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = args
        .iter()
        .map(|arg| {
            if arg.contains(' ') || arg.contains('\t') {
                format!("\"{}\"", arg.replace('"', "\\\""))
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, content)?;
    Ok(())
}

pub async fn spawn_process(command: &LaunchCommand) -> Result<Child, LaunchError> {
    spawn_process_with_log(command, None, None).await
}

pub async fn spawn_process_with_log(
    command: &LaunchCommand,
    log_dir: Option<&Path>,
    working_dir: Option<&Path>,
) -> Result<Child, LaunchError> {
    let mut cmd = Command::new(&command.program);
    cmd.args(&command.args).stdin(Stdio::null());

    if let Some(cwd) = working_dir {
        cmd.current_dir(cwd);
    }

    if let Some(dir) = log_dir {
        fs::create_dir_all(dir)?;
        let log_path = dir.join("launch.log");
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        let stderr = log_file.try_clone()?;
        cmd.stdout(Stdio::from(log_file)).stderr(Stdio::from(stderr));
    } else {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let child = cmd.spawn()?;
    Ok(child)
}

pub async fn early_exit_message(
    child: &mut Child,
    log_dir: Option<&Path>,
    timeout: Duration,
) -> Option<String> {
    tokio::time::sleep(timeout).await;
    let Ok(Some(status)) = child.try_wait() else {
        return None;
    };
    if status.success() {
        return Some("Java exited immediately after launch".into());
    }
    let code = status.code().map(|c| c.to_string()).unwrap_or_else(|| "?".into());
    let mut msg = format!("Java exited with code {code}");
    if let Some(dir) = log_dir {
        let log_path = dir.join("launch.log");
        if let Ok(tail) = read_log_tail(&log_path, 4096) {
            let trimmed = tail.trim();
            if !trimmed.is_empty() {
                msg.push_str(" — ");
                if trimmed.len() > 500 {
                    msg.push_str("...");
                    msg.push_str(&trimmed[trimmed.len() - 500..]);
                } else {
                    msg.push_str(trimmed);
                }
            }
        }
    }
    Some(msg)
}

fn read_log_tail(path: &Path, max_bytes: u64) -> Result<String, LaunchError> {
    let mut file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(start))?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("java executable path is required")]
    MissingJava,
    #[error("failed to spawn process: {0}")]
    Io(#[from] std::io::Error),
}
