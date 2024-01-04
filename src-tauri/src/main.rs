// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Context;
use ax_core::node::BindTo;

use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    let (ax_send, ax_rec) = mpsc::channel::<ActyxThreadParams>();
    let actyx_thread = std::thread::spawn(|| actyx_thread(ax_rec));

    tauri::Builder::default()
        .setup(move |app| {
            // Supply storage path to actyx
            let storage_dir = tauri::api::path::app_local_data_dir(&*app.config()).ok_or(
                anyhow::anyhow!("tauri::api::path::app_local_data_dir fails"),
            )?;
            let storage_dir = storage_dir.join("actyx");
            let bind_to = BindTo::random()?;
            ax_send.send(ActyxThreadParams {
                bind_to,
                storage_dir,
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    if let Err(x) = actyx_thread.join() {
        eprintln!("{:?}", x);
    }
}

struct ActyxThreadParams {
    storage_dir: PathBuf,
    bind_to: BindTo,
}

fn actyx_thread(rec_path: Receiver<ActyxThreadParams>) -> anyhow::Result<()> {
    use ax_core::node::{shutdown_ceremony, ApplicationState, Runtime};
    let ActyxThreadParams {
        bind_to,
        storage_dir,
    } = rec_path.recv()?;

    println!("actyx-data running in {}", storage_dir.display());
    std::fs::create_dir_all(storage_dir.clone())
        .with_context(|| format!("creating working directory `{:?}`", storage_dir.display()))?;
    let storage_dir = storage_dir.canonicalize()?;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let runtime = Runtime::Linux;
    #[cfg(target_os = "windows")]
    let runtime = Runtime::Windows;

    let actyx = ApplicationState::spawn(storage_dir, runtime, bind_to, false, false)?;

    /// Note to Jose: what I want to be able to do is something like this:
    /// ```
    /// actyx.manager.send(ExternalEvent::API::Query("FROM allEvents"));
    /// ```
    /// probably it is better to wrap it in a function that receive events as well

    shutdown_ceremony(actyx)?;

    Ok(())
}
