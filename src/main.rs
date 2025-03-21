// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use slint::{self, SharedString};
use rfd::FileDialog;
use librqbit::{AddTorrent, Session};
use std::path::PathBuf;
use tokio;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = AppWindow::new()?;

    app.on_select_directory({
        let app_weak = app.as_weak();
        move || {
            let app = app_weak.upgrade().unwrap();
            let dir = FileDialog::new().pick_folder();
            if let Some(dir) = dir {
                let dir_str = dir.to_string_lossy().to_string();
                app.set_download_dir(SharedString::from(dir_str));
            }
        }
    });

    app.on_start_download({
        let app_weak = app.as_weak();
        move || {
            let app = app_weak.upgrade().unwrap();
            let download_dir = app.get_download_dir().to_string();
            let magnet_link = app.get_magnet_link().to_string().trim().to_owned();

            if download_dir.is_empty() || magnet_link.is_empty() {
                app.set_status(SharedString::from("Please select a directory and enter a magnet link."));
                return;
            }

            let app_weak = app.as_weak();
            tokio::spawn(async move {
                let set_status = {
                    let app_weak = app_weak.clone();
                    move |msg: &str| {
                        let app_weak = app_weak.clone();
                        let msg = msg.to_string();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_weak.upgrade() {
                                app.set_status(SharedString::from(msg));
                            }
                        });
                    }
                };

                set_status("Initializing download session...");

                let session = match Session::new(PathBuf::from(&download_dir)).await {
                    Ok(session) => session,
                    Err(e) => {
                        set_status(&format!("Failed to initialize session: {}", e));
                        return;
                    }
                };

                set_status("Adding torrent to session...");

                let add_torrent_result = session.add_torrent(AddTorrent::from_url(&magnet_link), None).await;

                match add_torrent_result {
                    Ok(handle) => {
                        if let Some(managed_torrent_handle) = handle.into_handle() {
                            set_status("Torrent added successfully. Starting download...");

                            if let Err(e) = managed_torrent_handle.wait_until_completed().await {
                                set_status(&format!("Download failed: {}", e));
                            } else {
                                set_status("Download completed!");
                            }
                        } else {
                            set_status("Failed to get torrent handle");
                        }
                    }
                    Err(e) => {
                        set_status(&format!("Failed to add torrent: {}", e));
                    }
                }
            });
        }
    });

    app.run()?;

    Ok(())
}