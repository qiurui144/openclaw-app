use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;

/// 持久化安装会话，供断点续传使用。
/// 严格不包含 admin_password 字段。
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SessionState {
    pub source_mode_tag: String,
    pub local_zip_path: Option<String>,
    pub proxy_url: Option<String>,
    pub downloaded_files: Vec<DownloadedFile>,
    pub completed_step: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadedFile {
    pub key: String,
    pub path: String,
    pub sha256: String,
}

fn session_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".openclaw")
        .join("install_session.json")
}

pub fn load() -> Option<SessionState> {
    let path = session_path();
    if !path.exists() { return None; }
    let data = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&data).ok()
}

#[allow(dead_code)]
pub fn save(state: &SessionState) -> Result<()> {
    let path = session_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

pub fn clear(also_cleanup_tmp: Option<&str>) -> Result<()> {
    let path = session_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    if let Some(install_path) = also_cleanup_tmp {
        let tmp = PathBuf::from(install_path).join(".tmp");
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    static HOME_LOCK: Mutex<()> = Mutex::new(());

    fn with_temp_home<F: FnOnce()>(f: F) {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = env::temp_dir().join(format!("oc_test_{}_{:?}", std::process::id(), std::thread::current().id()));
        std::fs::create_dir_all(&tmp).unwrap();
        #[cfg(unix)]
        env::set_var("HOME", &tmp);
        f();
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        with_temp_home(|| {
            let state = SessionState {
                source_mode_tag: "Bundled".into(),
                completed_step: 3,
                ..Default::default()
            };
            save(&state).unwrap();
            let loaded = load().expect("should load");
            assert_eq!(loaded.source_mode_tag, "Bundled");
            assert_eq!(loaded.completed_step, 3);
        });
    }

    #[test]
    fn test_clear_removes_file() {
        with_temp_home(|| {
            let state = SessionState::default();
            save(&state).unwrap();
            clear(None).unwrap();
            assert!(load().is_none());
        });
    }

    #[test]
    fn test_load_returns_none_when_no_file() {
        with_temp_home(|| {
            assert!(load().is_none());
        });
    }

    #[test]
    fn test_no_password_field_in_serialized() {
        let state = SessionState::default();
        let json = serde_json::to_string(&state).unwrap();
        assert!(!json.to_lowercase().contains("password"));
        assert!(!json.to_lowercase().contains("passwd"));
    }
}
