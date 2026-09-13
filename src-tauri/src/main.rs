#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Path A: the entire Next.js app (API route, polling, everything) runs completely
// unchanged as a bundled Node.js "sidecar" process. This file's only job is to:
//   1. Start that sidecar on app launch.
//   2. Wait until it's actually accepting connections (avoids a flash of
//      "can't reach this page" while Node is still booting).
//   3. Open a native window pointed at it.
//   4. Kill the sidecar when the window closes, so it never lingers in the
//      background after the app quits.
//
// NOTE: this was written and reasoned through carefully, but could not be
// compiled/tested here (no Rust/Cargo/Windows available in this environment).
// Treat it as a strong first draft — run `cargo check` locally and diff
// against whatever the installed `tauri` / `tauri-plugin-shell` version's
// docs say if anything fails to compile; the shell-plugin API has shifted
// slightly across Tauri 2.x minor versions.

use std::net::TcpStream;
use std::sync::Mutex;
use std::time::Duration;

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_shell::process::CommandChild;
use tauri_plugin_shell::ShellExt;

// Change this if 3939 is ever likely to collide with something else on a
// user's machine. Must match the PORT env var passed to the sidecar below.
const SERVER_PORT: u16 = 3939;

/// Holds the sidecar's child process handle so it can be killed on exit.
struct SidecarHandle(Mutex<Option<CommandChild>>);

/// Gracefully kills the background server sidecar process.
/// Called by the frontend right before applying an update and restarting.
#[tauri::command]
fn stop_server(state: tauri::State<SidecarHandle>) {
    if let Some(child) = state.0.lock().unwrap().take() {
        let _ = child.kill();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(SidecarHandle(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![stop_server])
        .setup(|app| {
            // If an orphaned sidecar from an earlier run or a killed dev terminal session
            // is still running on SERVER_PORT, terminate it before spawning a new one.
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;

                // 1. Force kill any existing zombie sidecar binary from previous dev sessions
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/IM", "server-x86_64-pc-windows-msvc.exe", "/T"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .status();

                // 2. If port 3939 is still occupied, free it
                if TcpStream::connect(format!("127.0.0.1:{}", SERVER_PORT)).is_ok() {
                    let _ = std::process::Command::new("powershell")
                        .args([
                            "-ExecutionPolicy",
                            "Bypass",
                            "-NoProfile",
                            "-Command",
                            &format!(
                                "try {{ $p = (Get-NetTCPConnection -LocalPort {} -State Listen -ErrorAction Stop).OwningProcess; Stop-Process -Id $p -Force }} catch {{}}; exit 0",
                                SERVER_PORT
                            ),
                        ])
                        .creation_flags(CREATE_NO_WINDOW)
                        .status();

                    std::thread::sleep(Duration::from_millis(300));
                }
            }

            // Resolve the bundled server.js inside the installed app's
            // resource directory (this is where `bundle.resources` in
            // tauri.conf.json gets copied to at install time).
            let server_js = app
                .path()
                .resolve("resources/server/server.js", tauri::path::BaseDirectory::Resource)?;

            let shell = app.shell();
            let (mut rx, child) = shell
                .sidecar("server")? // must match the name in bundle.externalBin
                .args([server_js.to_string_lossy().to_string()])
                .env("PORT", SERVER_PORT.to_string())
                // Bind to loopback only — this app has no reason to be
                // reachable from other devices on the network, and it
                // narrows the Windows Firewall prompt to localhost-only.
                .env("HOSTNAME", "127.0.0.1")
                .spawn()?;

            app.state::<SidecarHandle>()
                .0
                .lock()
                .unwrap()
                .replace(child);

            // Drain the sidecar's stdout/stderr. If nothing reads these,
            // the pipe can eventually fill up and stall the Node process.
            tauri::async_runtime::spawn(async move {
                use tauri_plugin_shell::process::CommandEvent;
                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stdout(line) => {
                            println!("[server] {}", String::from_utf8_lossy(&line));
                        }
                        CommandEvent::Stderr(line) => {
                            eprintln!("[server] {}", String::from_utf8_lossy(&line));
                        }
                        _ => {}
                    }
                }
            });

            // Poll localhost until the Next.js server actually answers,
            // then open the window. 100 x 150ms = 15s max wait before
            // giving up and opening anyway (better than hanging forever
            // if something's genuinely wrong — the page will just show a
            // connection error the user can report).
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let addr = format!("127.0.0.1:{}", SERVER_PORT);
                for _ in 0..100 {
                    if TcpStream::connect(&addr).is_ok() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(150)).await;
                }

                let url = format!("http://127.0.0.1:{}", SERVER_PORT);
                let _ = WebviewWindowBuilder::new(
                    &app_handle,
                    "main",
                    WebviewUrl::External(url.parse().expect("invalid sidecar URL")),
                )
                .title("VaLight Tracker")
                .inner_size(1100.0, 760.0)
                .min_inner_size(820.0, 560.0)
                .build();
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Kill the sidecar the moment the window is closing — otherwise
            // the Node process (and its 8-15s poll loop hitting Riot's API)
            // keeps running invisibly in the background after the app quits.
            if matches!(
                event,
                tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
            ) {
                if let Some(state) = window.try_state::<SidecarHandle>() {
                    if let Some(child) = state.0.lock().unwrap().take() {
                        let _ = child.kill();
                    }
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building VaLight Tracker")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app_handle.try_state::<SidecarHandle>() {
                    if let Some(child) = state.0.lock().unwrap().take() {
                        let _ = child.kill();
                    }
                }
            }
        });
}
