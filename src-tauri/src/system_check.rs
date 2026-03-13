use serde::Serialize;
use std::net::TcpListener;

#[derive(Debug, Serialize, Clone)]
pub struct CheckItem {
    pub name: String,
    pub detail: String,
    pub passed: bool,
    pub required: bool,
}

pub async fn run_all_checks() -> Vec<CheckItem> {
    vec![
        check_os_version(),
        check_disk_space(),
        check_port_available(18789),
    ]
}

fn check_os_version() -> CheckItem {
    #[cfg(target_os = "linux")]
    {
        let detail = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|s| {
                let pretty = s.lines()
                    .find(|l| l.starts_with("PRETTY_NAME="))
                    .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string());
                pretty
            })
            .unwrap_or_else(|| "未知 Linux 发行版".into());

        let kernel = std::process::Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();

        let major: u32 = kernel.split('.').next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let passed = major >= 5;

        return CheckItem {
            name: "操作系统版本".into(),
            detail: format!("{} (内核 {})", detail, kernel),
            passed,
            required: true,
        };
    }

    #[cfg(target_os = "windows")]
    {
        let build: u32 = {
            let out = std::process::Command::new("reg")
                .args(["query",
                    r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion",
                    "/v", "CurrentBuildNumber"])
                .output()
                .ok();
            out.and_then(|o| {
                String::from_utf8(o.stdout).ok()
                    .and_then(|s| s.lines()
                        .find(|l| l.contains("CurrentBuildNumber"))
                        .and_then(|l| l.split_whitespace().last()
                            .and_then(|v| v.parse().ok())))
            }).unwrap_or(0)
        };
        let passed = build >= 17134;
        let label = if build >= 22000 { "Windows 11" } else { "Windows 10" };
        return CheckItem {
            name: "操作系统版本".into(),
            detail: format!("{} (Build {})", label, build),
            passed,
            required: true,
        };
    }

    #[cfg(target_os = "macos")]
    {
        return CheckItem {
            name: "操作系统版本".into(),
            detail: "macOS（bash 脚本模式，不适用）".into(),
            passed: true,
            required: true,
        };
    }
}

fn check_disk_space() -> CheckItem {
    let path = dirs::home_dir()
        .filter(|p| p.exists())
        .unwrap_or_else(|| std::path::PathBuf::from("/"));

    let available_mb: u64 = {
        #[cfg(unix)]
        {
            use std::ffi::CString;
            let path_cstr = CString::new(path.to_str().unwrap_or("/")).unwrap_or_default();
            let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
            let ret = unsafe { libc::statvfs(path_cstr.as_ptr(), &mut stat) };
            if ret == 0 {
                (stat.f_bavail as u64 * stat.f_frsize as u64) / (1024 * 1024)
            } else {
                0
            }
        }
        #[cfg(windows)]
        { 0 }
    };

    let passed = available_mb >= 512;
    CheckItem {
        name: "磁盘空间".into(),
        detail: format!("可用 {} MB（需要 ≥ 512 MB）", available_mb),
        passed,
        required: true,
    }
}

fn check_port_available(port: u16) -> CheckItem {
    let passed = TcpListener::bind(format!("127.0.0.1:{}", port)).is_ok();
    CheckItem {
        name: format!("端口 {} 可用", port),
        detail: if passed {
            format!("端口 {} 未被占用", port)
        } else {
            format!("端口 {} 已被占用，将在部署时尝试停止冲突服务", port)
        },
        passed,
        required: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_item_serializable() {
        let item = CheckItem {
            name: "测试".into(),
            detail: "详情".into(),
            passed: true,
            required: true,
        };
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("测试"));
    }

    #[test]
    fn test_disk_space_check_returns_item() {
        let item = check_disk_space();
        assert!(!item.name.is_empty());
        assert!(item.required);
        assert!(item.passed, "测试环境磁盘应有足够空间");
    }

    #[test]
    fn test_port_check_high_port_likely_free() {
        let item = check_port_available(59999);
        assert_eq!(item.required, false);
        assert!(item.passed, "59999 端口应该未被占用");
    }
}
