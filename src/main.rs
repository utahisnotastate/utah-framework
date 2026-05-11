use tao::{
    event::{Event, TrayEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    menu::{ContextMenu, MenuItemAttributes},
    system_tray::SystemTrayBuilder,
    window::Icon,
    window::WindowBuilder,
};
use wry::webview::WebViewBuilder;
use serde::{Deserialize, Serialize};
use std::thread;
use std::sync::mpsc;
use std::time::Duration;
use std::fs;
use std::io::{Read, Write};
use rusqlite::Connection;
use keyring::Entry;
use self_update::Status;
use arboard::Clipboard;
use notify_rust::Notification;
use rfd::FileDialog;

#[derive(Deserialize, Debug)]
struct IpcMessage {
    channel: String,
    data: serde_json::Value,
}

#[derive(Serialize, Debug, Clone)]
struct IpcResponse {
    channel: String,
    status: String,
    payload: String,
}

fn is_vite_server(port: u16) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};

    let addr = ("127.0.0.1", port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut iter| iter.next());

    let Some(addr) = addr else {
        return false;
    };

    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_millis(400)) {
        Ok(s) => s,
        Err(_) => return false,
    };

    let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(400)));

    if stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }

    let mut buf = vec![0u8; 8192];
    let mut total = 0usize;
    loop {
        let n = match stream.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        total += n;
        if total >= buf.len() {
            break;
        }
    }

    if total == 0 {
        return false;
    }

    let response = String::from_utf8_lossy(&buf[..total]);
    let lower = response.to_lowercase();
    let ok = lower.contains("200 ok")
        || lower.lines().next().is_some_and(|l| l.contains(" 200 "));
    if !ok {
        return false;
    }
    lower.contains("vite")
        || lower.contains("@vite")
        || lower.contains("main.jsx")
        || lower.contains("/@fs/")
        || lower.contains("/@id/")
        || (lower.contains("<!doctype html") && lower.len() > 120)
}

/// Prefer the highest open Vite port (avoids stale listeners on 5173 when Vite moved to 5176).
fn best_vite_port_in_range() -> Option<u16> {
    (5173u16..=5200).filter(|&p| is_vite_server(p)).max()
}

fn read_vite_port_file() -> Option<u16> {
    std::fs::read_to_string(".utah-vite-port")
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Opaque RGBA pixels for `tao::window::Icon` (R, G, B, A per pixel).
fn tray_icon_rgba() -> Vec<u8> {
    const W: usize = 32;
    const H: usize = 32;
    let mut v = Vec::with_capacity(W * H * 4);
    for y in 0..H {
        for x in 0..W {
            let on = ((x / 4) + (y / 4)) % 2 == 0;
            if on {
                v.extend_from_slice(&[14, 165, 233, 255]); // sky #0ea5e9
            } else {
                v.extend_from_slice(&[30, 41, 59, 255]); // slate #1e293b
            }
        }
    }
    v
}

fn resolve_vite_dev_url() -> String {
    if let Some(port) = read_vite_port_file() {
        for attempt in 1..=40 {
            if is_vite_server(port) {
                println!(
                    "[ZEO-ARCHITECT] Using .utah-vite-port -> http://localhost:{} (attempt {})",
                    port, attempt
                );
                return format!("http://localhost:{port}");
            }
            if attempt == 1 {
                println!(
                    "[ZEO-ARCHITECT] Waiting for Vite on port from .utah-vite-port ({}) ...",
                    port
                );
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    if let Ok(raw) = std::env::var("UTAH_VITE_PORT") {
        if let Ok(port) = raw.trim().parse::<u16>() {
            for attempt in 1..=120 {
                if is_vite_server(port) {
                    println!(
                        "[ZEO-ARCHITECT] Using UTAH_VITE_PORT -> http://localhost:{} (after {} waits)",
                        port, attempt
                    );
                    return format!("http://localhost:{port}");
                }
                if attempt == 1 {
                    println!(
                        "[ZEO-ARCHITECT] Waiting for Vite on UTAH_VITE_PORT={} ...",
                        port
                    );
                }
                thread::sleep(Duration::from_millis(250));
            }
            eprintln!(
                "[UTAH] UTAH_VITE_PORT={} never responded as Vite; scanning 5173..5200.",
                port
            );
        }
    }

    for attempt in 1..=120 {
        if let Some(port) = best_vite_port_in_range() {
            println!(
                "[ZEO-ARCHITECT] Resonance Target Locked: http://localhost:{} (scan #{})",
                port, attempt
            );
            return format!("http://localhost:{port}");
        }
        if attempt == 1 {
            println!(
                "[ZEO-ARCHITECT] Waiting for Vite on 127.0.0.1:5173..5200 (run `npm run dev:ui` first, or `npm run dev`)."
            );
        } else if attempt % 8 == 0 {
            println!(
                "[ZEO-ARCHITECT] Still waiting for Vite... (~{}s). Tip: set UTAH_VITE_PORT if you use a custom port.",
                attempt / 4
            );
        }
        thread::sleep(Duration::from_millis(250));
    }

    eprintln!(
        "[UTAH] Vite not detected after 30s; loading http://localhost:5173 anyway (blank until Vite is up)."
    );
    "http://localhost:5173".to_string()
}

/// GitHub Releases self-update (Delta-Wave). Configure via env:
/// `UTAH_UPDATE_REPO_OWNER`, `UTAH_UPDATE_REPO_NAME`, `UTAH_UPDATE_BIN_NAME` (default `utah-core`).
/// Optional: `GITHUB_TOKEN` for higher API limits / private repos.
fn run_github_self_update() -> Result<Status, String> {
    let owner = std::env::var("UTAH_UPDATE_REPO_OWNER").unwrap_or_else(|_| "Utah-1".into());
    let repo = std::env::var("UTAH_UPDATE_REPO_NAME").unwrap_or_else(|_| "utah-framework".into());
    let bin_name = std::env::var("UTAH_UPDATE_BIN_NAME").unwrap_or_else(|_| "utah-core".into());

    let updater = self_update::backends::github::Update::configure()
        .repo_owner(&owner)
        .repo_name(&repo)
        .bin_name(&bin_name)
        .show_download_progress(false)
        .current_version(env!("CARGO_PKG_VERSION"))
        .build()
        .map_err(|e| format!("{e}"))?;

    updater.update().map_err(|e| format!("{e}"))
}

/// Safe `evaluate_script` payload: JSON-encode each argument so quotes/newlines cannot break JS.
fn ipc_response_to_eval_js(response: &IpcResponse) -> String {
    let ch = serde_json::to_string(&response.channel).unwrap_or_else(|_| "\"\"".into());
    let st = serde_json::to_string(&response.status).unwrap_or_else(|_| "\"\"".into());
    let pl = serde_json::to_string(&response.payload).unwrap_or_else(|_| "\"\"".into());
    format!("window.Utah.receiveResponse({}, {}, {});", ch, st, pl)
}

fn main() -> wry::Result<()> {
    // Initialize EventLoop; we’ll forward worker results via a channel into the UI thread
    let event_loop = EventLoop::new();
    let wake_worker = event_loop.create_proxy();

    // THE FRAMELESS MATRIX (opaque in dev: transparent + WebView2 often reads as a blank window)
    let mut window_builder = WindowBuilder::new()
        .with_title("Utah Protocol [L6: Phantom Daemon]")
        .with_inner_size(tao::dpi::LogicalSize::new(900.0, 600.0))
        .with_decorations(false); // Strips OS titlebar and borders

    #[cfg(debug_assertions)]
    {
        window_builder = window_builder.with_transparent(false);
    }
    #[cfg(not(debug_assertions))]
    {
        window_builder = window_builder.with_transparent(true);
    }

    let window = window_builder.build(&event_loop).unwrap();

    // PHANTOM DAEMON (SYSTEM TRAY) INITIATION
    let mut tray_menu = ContextMenu::new();
    let show_id = tray_menu
        .add_item(MenuItemAttributes::new("Awaken Utah Core"))
        .id();
    let quit_id = tray_menu
        .add_item(MenuItemAttributes::new("Terminate Process"))
        .id();

    // Windows tray: use a fully opaque 32×32 RGBA pattern (uniform semi-transparent icons often disappear)
    let tray_icon = Icon::from_rgba(tray_icon_rgba(), 32, 32).expect("tray icon rgba");
    let _system_tray = SystemTrayBuilder::new(tray_icon, Some(tray_menu))
        .build(&event_loop)
        .expect("system tray");
    println!("[UTAH] System tray active (look in the notification area / “^” overflow if hidden).");

    // Channel: UI (ipc_handler) -> worker
    let (tx, rx) = mpsc::channel::<IpcMessage>();
    // Channel: worker -> UI (event loop)
    let (tx_resp, rx_resp) = mpsc::channel::<IpcResponse>();
    let wake_clone = wake_worker.clone();

    // Aegis Background Worker Pool
    thread::spawn(move || {
        println!("[UTAH-WORKER] Phantom Daemon Online. Awaiting background telemetry.");
        let conn = Connection::open("utah_data.db").expect("Failed to bind SQLite");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)",
            (),
        )
        .expect("Failed to initialize users table");

        for msg in rx {
            match msg.channel.as_str() {
                "system:ping" => {
                    thread::sleep(Duration::from_millis(500)); 
                    println!("[UTAH-WORKER] Processed Ping. Sending response to UI...");
                    
                    let res = IpcResponse {
                        channel: msg.channel.clone(),
                        status: "SUCCESS".to_string(),
                        payload: format!("Core Synapse synchronized at {}", msg.data["timestamp"]),
                    };
                    let _ = tx_resp.send(res);
                    let _ = wake_clone.send_event(());
                },
                "system:update" => {
                    println!("[UTAH-WORKER] Delta-Wave: scanning GitHub Releases...");
                    let res = match run_github_self_update() {
                        Ok(status) => {
                            if status.updated() {
                                IpcResponse {
                                    channel: msg.channel.clone(),
                                    status: "UPDATED".to_string(),
                                    payload: format!(
                                        "Installed v{}. Restart the application to run the new binary.",
                                        status.version()
                                    ),
                                }
                            } else {
                                IpcResponse {
                                    channel: msg.channel.clone(),
                                    status: "UP_TO_DATE".to_string(),
                                    payload: format!(
                                        "Already current (checked against v{}).",
                                        status.version()
                                    ),
                                }
                            }
                        }
                        Err(e) => IpcResponse {
                            channel: msg.channel.clone(),
                            status: "ERROR".to_string(),
                            payload: e,
                        },
                    };
                    let _ = tx_resp.send(res);
                    let _ = wake_clone.send_event(());
                },
                "fs:read" => {
                    let path = msg.data["path"].as_str().unwrap_or("");
                    println!("[UTAH-WORKER] Native File System Access Granted: {}", path);
                    
                    let content = match fs::read_to_string(path) {
                        Ok(c) => c,
                        Err(e) => format!("ACCESS_ERROR: {}", e),
                    };
                    
                    let res = IpcResponse {
                        channel: msg.channel.clone(),
                        status: "COMPLETED".to_string(),
                        payload: content,
                    };
                    let _ = tx_resp.send(res);
                    let _ = wake_clone.send_event(());
                },
                "db:execute" => {
                    let query = msg.data["query"].as_str().unwrap_or("");
                    let res = match conn.execute(query, ()) {
                        Ok(rows) => IpcResponse {
                            channel: msg.channel.clone(),
                            status: "SUCCESS".to_string(),
                            payload: format!("Rows affected: {}", rows),
                        },
                        Err(e) => IpcResponse {
                            channel: msg.channel.clone(),
                            status: "ERROR".to_string(),
                            payload: e.to_string(),
                        },
                    };
                    let _ = tx_resp.send(res);
                    let _ = wake_clone.send_event(());
                },
                "store:set_secure" => {
                    let key = msg.data["key"].as_str().unwrap_or("");
                    let val = msg.data["value"].as_str().unwrap_or("");
                    let res = match Entry::new("utah_app", key) {
                        Ok(entry) => match entry.set_password(val) {
                            Ok(_) => IpcResponse {
                                channel: msg.channel.clone(),
                                status: "SUCCESS".to_string(),
                                payload: "SECURED".to_string(),
                            },
                            Err(e) => IpcResponse {
                                channel: msg.channel.clone(),
                                status: "ERROR".to_string(),
                                payload: e.to_string(),
                            },
                        },
                        Err(e) => IpcResponse {
                            channel: msg.channel.clone(),
                            status: "ERROR".to_string(),
                            payload: e.to_string(),
                        },
                    };
                    let _ = tx_resp.send(res);
                    let _ = wake_clone.send_event(());
                },
                "store:get_secure" => {
                    let key = msg.data["key"].as_str().unwrap_or("");
                    let res = match Entry::new("utah_app", key) {
                        Ok(entry) => match entry.get_password() {
                            Ok(val) => IpcResponse {
                                channel: msg.channel.clone(),
                                status: "SUCCESS".to_string(),
                                payload: val,
                            },
                            Err(_) => IpcResponse {
                                channel: msg.channel.clone(),
                                status: "SUCCESS".to_string(),
                                payload: "NOT_FOUND".to_string(),
                            },
                        },
                        Err(e) => IpcResponse {
                            channel: msg.channel.clone(),
                            status: "ERROR".to_string(),
                            payload: e.to_string(),
                        },
                    };
                    let _ = tx_resp.send(res);
                    let _ = wake_clone.send_event(());
                },
                "dialog:open" => {
                    println!("[UTAH-WORKER] Spawning Native OS File Dialog.");
                    let path = FileDialog::new().pick_file();
                    let payload = match path {
                        Some(p) => p.display().to_string(),
                        None => "CANCELLED".to_string(),
                    };
                    let res = IpcResponse {
                        channel: msg.channel.clone(),
                        status: "SUCCESS".to_string(),
                        payload,
                    };
                    let _ = tx_resp.send(res);
                    let _ = wake_clone.send_event(());
                },
                "clipboard:write" => {
                    let text = msg.data["text"].as_str().unwrap_or("");
                    let res = match Clipboard::new() {
                        Ok(mut clipboard) => match clipboard.set_text(text) {
                            Ok(_) => IpcResponse {
                                channel: msg.channel.clone(),
                                status: "SUCCESS".to_string(),
                                payload: "COPIED".to_string(),
                            },
                            Err(e) => IpcResponse {
                                channel: msg.channel.clone(),
                                status: "ERROR".to_string(),
                                payload: e.to_string(),
                            },
                        },
                        Err(e) => IpcResponse {
                            channel: msg.channel.clone(),
                            status: "ERROR".to_string(),
                            payload: e.to_string(),
                        },
                    };
                    let _ = tx_resp.send(res);
                    let _ = wake_clone.send_event(());
                },
                "clipboard:read" => {
                    let res = match Clipboard::new() {
                        Ok(mut clipboard) => IpcResponse {
                            channel: msg.channel.clone(),
                            status: "SUCCESS".to_string(),
                            payload: clipboard.get_text().unwrap_or_default(),
                        },
                        Err(e) => IpcResponse {
                            channel: msg.channel.clone(),
                            status: "ERROR".to_string(),
                            payload: e.to_string(),
                        },
                    };
                    let _ = tx_resp.send(res);
                    let _ = wake_clone.send_event(());
                },
                "system:notify" => {
                    let title = msg.data["title"].as_str().unwrap_or("Utah Alert");
                    let body = msg.data["body"].as_str().unwrap_or("");
                    let notify_res = Notification::new().summary(title).body(body).show();
                    let res = match notify_res {
                        Ok(_) => IpcResponse {
                            channel: msg.channel.clone(),
                            status: "SUCCESS".to_string(),
                            payload: "DISPATCHED".to_string(),
                        },
                        Err(e) => IpcResponse {
                            channel: msg.channel.clone(),
                            status: "ERROR".to_string(),
                            payload: e.to_string(),
                        },
                    };
                    let _ = tx_resp.send(res);
                    let _ = wake_clone.send_event(());
                },
                _ => println!("[UTAH-WORKER] Unhandled command matrix."),
            }
        }
    });

    // NATIVE WINDOW CONTROLLER INTERCEPTOR
    let ipc_tx = tx.clone();
    let ipc_handler = move |window: &tao::window::Window, req: String| {
        if let Ok(msg) = serde_json::from_str::<IpcMessage>(&req) {
            match msg.channel.as_str() {
                "window:minimize" => window.set_minimized(true),
                "window:maximize" => window.set_maximized(!window.is_maximized()),
                "window:close" => {
                    println!("[ZEO-ARCHITECT] Quit requested from UI.");
                    std::process::exit(0);
                }
                "window:drag" => {
                    let _ = window.drag_window();
                }
                _ => {
                    let _ = ipc_tx.send(msg);
                }
            }
        }
    };

    // --- THE BIFURCATION MATRIX (DEV vs PROD) ---
    let mut webview_builder = WebViewBuilder::new(window)?
        .with_ipc_handler(ipc_handler)
        .with_initialization_script(include_str!("../public/utah_bridge.js"));

    #[cfg(debug_assertions)]
    {
        webview_builder = webview_builder.with_transparent(false).with_devtools(true);
    }
    #[cfg(not(debug_assertions))]
    {
        webview_builder = webview_builder.with_transparent(true);
    }

    #[cfg(debug_assertions)]
    {
        let dev_url = resolve_vite_dev_url();
        webview_builder = webview_builder.with_url(&dev_url)?;
    }

    #[cfg(not(debug_assertions))]
    {
        // In Production: Swallow the compiled React app as a single string
        println!("[ZEO-ARCHITECT] Production Matrix Active. Forging Standalone Executable...");
        let html_content = include_str!("../dist/index.html");
        webview_builder = webview_builder.with_html(html_content)?;
    }

    let webview = webview_builder.build()?;

    webview.window().set_visible(true);
    let _ = webview.window().set_focus();

    #[cfg(debug_assertions)]
    println!("[ZEO-ARCHITECT] WebView devtools enabled (debug build).");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // Drain any pending responses from worker and inject into JS
        while let Ok(response) = rx_resp.try_recv() {
            let js = ipc_response_to_eval_js(&response);
            if let Err(e) = webview.evaluate_script(&js) {
                eprintln!("[UTAH] evaluate_script failed: {e}");
            }
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("[ZEO-ARCHITECT] Close requested — exiting.");
                *control_flow = ControlFlow::Exit;
            }
            Event::TrayEvent {
                event: TrayEvent::LeftClick,
                ..
            } => {
                println!("[ZEO-ARCHITECT] Awaking UI from Phantom Sleep.");
                webview.window().set_visible(true);
                webview.window().set_focus();
            }
            Event::MenuEvent { menu_id, .. } => {
                if menu_id == quit_id {
                    println!("[ZEO-ARCHITECT] Absolute Process Termination Commencing.");
                    *control_flow = ControlFlow::Exit;
                } else if menu_id == show_id {
                    println!("[ZEO-ARCHITECT] Awaking UI from Phantom Sleep.");
                    webview.window().set_visible(true);
                    webview.window().set_focus();
                }
            }
            _ => (),
        }
    });
}