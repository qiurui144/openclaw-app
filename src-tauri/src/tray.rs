/// 系统托盘模块：图标、菜单、30s 轮询服务状态
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use crate::service_ctrl::{DeployMeta, ServiceStatus};

// ── 托盘状态（存储当前已知的安装 meta，可在部署后动态更新）────────────────────

pub struct TrayMeta(pub Mutex<Option<DeployMeta>>);

// ── 初始化 ────────────────────────────────────────────────────────────────────

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let meta_opt = crate::service_ctrl::read_meta();

    // 先注册 state，保证 on_window_event 能读到
    app.manage(TrayMeta(Mutex::new(meta_opt.clone())));

    let menu = build_menu(app, false, meta_opt.is_some())?;

    TrayIconBuilder::with_id("openclaw-tray")
        .tooltip("OpenClaw 部署向导")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            // 左键单击：显示主窗口
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .on_menu_event(|app, event| handle_menu(app, event.id.as_ref()))
        .build(app)?;

    // 启动 30s 轮询
    let handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        poll_loop(handle).await;
    });

    Ok(())
}

// ── 菜单构建（每次轮询重建，简单可靠）────────────────────────────────────────

fn build_menu(
    handle: &impl Manager<tauri::Wry>,
    is_running: bool,
    has_meta: bool,
) -> tauri::Result<Menu<tauri::Wry>> {
    let status_text = if !has_meta {
        "未安装 OpenClaw"
    } else if is_running {
        "状态: 运行中 🟢"
    } else {
        "状态: 已停止 🔴"
    };

    Menu::with_items(
        handle,
        &[
            &MenuItem::with_id(handle, "status",  status_text,    false,                    None::<&str>)?,
            &PredefinedMenuItem::separator(handle)?,
            &MenuItem::with_id(handle, "start",   "▶ 启动",        has_meta && !is_running, None::<&str>)?,
            &MenuItem::with_id(handle, "stop",    "⏹ 停止",        has_meta && is_running,  None::<&str>)?,
            &MenuItem::with_id(handle, "restart", "🔄 重启",        has_meta,                None::<&str>)?,
            &PredefinedMenuItem::separator(handle)?,
            &MenuItem::with_id(handle, "console", "🌐 打开控制台",  has_meta,                None::<&str>)?,
            &PredefinedMenuItem::separator(handle)?,
            &MenuItem::with_id(handle, "wizard",  "🔧 部署向导",   true,                    None::<&str>)?,
            &PredefinedMenuItem::separator(handle)?,
            &MenuItem::with_id(handle, "quit",    "❌ 退出",        true,                    None::<&str>)?,
        ],
    )
}

// ── 菜单事件处理 ──────────────────────────────────────────────────────────────

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "start" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let meta = app.state::<TrayMeta>().0.lock().unwrap().clone();
                if let Some(m) = meta {
                    if let Err(e) = crate::service_ctrl::start(&m) {
                        eprintln!("[tray] 启动失败: {e}");
                    }
                    // 延迟后刷新状态
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    refresh_tray_icon(&app).await;
                }
            });
        }
        "stop" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = crate::service_ctrl::stop() {
                    eprintln!("[tray] 停止失败: {e}");
                }
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                refresh_tray_icon(&app).await;
            });
        }
        "restart" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ = crate::service_ctrl::stop();
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                let meta = app.state::<TrayMeta>().0.lock().unwrap().clone();
                if let Some(m) = meta {
                    let _ = crate::service_ctrl::start(&m);
                }
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                refresh_tray_icon(&app).await;
            });
        }
        "console" => {
            let meta = app.state::<TrayMeta>().0.lock().unwrap().clone();
            if let Some(m) = meta {
                let _ = open::that(format!("http://127.0.0.1:{}", m.service_port));
            }
        }
        "wizard" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

// ── 轮询循环 ──────────────────────────────────────────────────────────────────

async fn poll_loop(app: AppHandle) {
    // 首次刷新（启动后 3s 内完成第一次探测）
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    refresh_tray_icon(&app).await;

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
    loop {
        interval.tick().await;
        refresh_tray_icon(&app).await;
    }
}

// ── 刷新托盘图标和菜单（部署完成后也可主动调用）──────────────────────────────

pub async fn refresh_tray_icon(app: &AppHandle) {
    // 若 meta 尚未加载（刚完成部署），重新读取
    {
        let state = app.state::<TrayMeta>();
        let mut guard = state.0.lock().unwrap();
        if guard.is_none() {
            *guard = crate::service_ctrl::read_meta();
        }
    }

    let state = app.state::<TrayMeta>();
    let meta = state.0.lock().unwrap().clone();
    let (is_running, has_meta) = if let Some(ref m) = meta {
        let status = crate::service_ctrl::check_status(m.service_port).await;
        (status == ServiceStatus::Running, true)
    } else {
        (false, false)
    };

    let tooltip = if is_running {
        "OpenClaw · 运行中 🟢"
    } else if has_meta {
        "OpenClaw · 已停止 🔴"
    } else {
        "OpenClaw 部署向导"
    };

    if let Some(tray) = app.tray_by_id("openclaw-tray") {
        let _ = tray.set_tooltip(Some(tooltip));
        if let Ok(menu) = build_menu(app, is_running, has_meta) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}
