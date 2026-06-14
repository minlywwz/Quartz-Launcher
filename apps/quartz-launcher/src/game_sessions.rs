use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::Serialize;
use tokio::process::Child;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningSessionInfo {
    pub session_id: String,
    pub instance_id: String,
    pub instance_name: String,
    pub pid: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceRunState {
    pub instance_id: String,
    pub running: bool,
    pub session_count: usize,
    pub sessions: Vec<RunningSessionInfo>,
}

struct SessionEntry {
    info: RunningSessionInfo,
    child: Arc<Mutex<Child>>,
}

#[derive(Clone, Default)]
pub struct GameSessions {
    inner: Arc<RwLock<HashMap<String, SessionEntry>>>,
}

impl std::fmt::Debug for GameSessions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameSessions")
            .field("active_sessions", &self.total_running())
            .finish()
    }
}

impl GameSessions {
    pub fn register(
        &self,
        instance_id: impl Into<String>,
        instance_name: impl Into<String>,
        child: Child,
    ) -> RunningSessionInfo {
        let session_id = Uuid::new_v4().to_string();
        let instance_id = instance_id.into();
        let instance_name = instance_name.into();
        let pid = child.id().unwrap_or(0);
        let info = RunningSessionInfo {
            session_id: session_id.clone(),
            instance_id: instance_id.clone(),
            instance_name,
            pid,
        };
        let child = Arc::new(Mutex::new(child));

        self.inner.write().insert(
            session_id.clone(),
            SessionEntry {
                info: info.clone(),
                child: child.clone(),
            },
        );

        let sessions = self.clone();
        tokio::spawn(async move {
            let mut guard = child.lock().await;
            let _ = guard.wait().await;
            drop(guard);
            sessions.inner.write().remove(&session_id);
        });

        info
    }

    pub fn instance_state(&self, instance_id: &str) -> InstanceRunState {
        let sessions: Vec<RunningSessionInfo> = self
            .inner
            .read()
            .values()
            .filter(|entry| entry.info.instance_id == instance_id)
            .map(|entry| entry.info.clone())
            .collect();
        let session_count = sessions.len();
        InstanceRunState {
            instance_id: instance_id.to_owned(),
            running: session_count > 0,
            session_count,
            sessions,
        }
    }

    pub fn total_running(&self) -> usize {
        self.inner.read().len()
    }

    pub async fn stop_all_for_instance(&self, instance_id: &str) -> Result<u32, String> {
        let ids: Vec<String> = self
            .inner
            .read()
            .values()
            .filter(|entry| entry.info.instance_id == instance_id)
            .map(|entry| entry.info.session_id.clone())
            .collect();

        let mut stopped = 0u32;
        for id in ids {
            if self.stop_session(&id).await? {
                stopped += 1;
            }
        }
        Ok(stopped)
    }

    pub async fn stop_session(&self, session_id: &str) -> Result<bool, String> {
        let entry = self.inner.write().remove(session_id);
        let Some(entry) = entry else {
            return Ok(false);
        };
        let mut child = entry.child.lock().await;
        child
            .kill()
            .await
            .map_err(|e| format!("failed to stop game process: {e}"))?;
        Ok(true)
    }
}
