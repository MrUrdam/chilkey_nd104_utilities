//! Commands for the keymap backup: save all layers and the knob to a file,
//! and restore such a file (only the differences are written, then
//! everything is read back and compared). Runs through the engine like
//! every other keyboard access.

use nd104_core::device::DeviceState;
use nd104_core::engine::EngineCommand;
use nd104_core::keymap::{Keymap, KeymapBackup, KeymapOp, KeymapResult, KeymapSpec};
use nd104_core::model::{self, Role};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::{mpsc, oneshot};

use crate::AppState;

type CmdResult<T> = Result<T, String>;

/// A checked backup file, kept until it is restored or replaced.
pub(crate) struct PendingRestore {
    backup: KeymapBackup,
}

/// Connected keyboard: model id, keymap size, keyboard firmware text.
pub(crate) fn connected(app: &AppHandle) -> CmdResult<(String, KeymapSpec, String)> {
    let status = app.state::<AppState>().status.borrow().clone();
    let DeviceState::Ready(device) = status.device else {
        return Err("Connect the keyboard with the USB cable first.".into());
    };
    let spec = model::registry()
        .iter()
        .find(|m| m.id() == device.model_id)
        .and_then(|m| m.keymap_spec())
        .ok_or("This keyboard does not support keymap backups.")?;
    let firmware = device
        .firmware
        .iter()
        .find(|f| f.role == Role::Keyboard)
        .map(|f| f.raw.clone())
        .unwrap_or_default();
    Ok((device.model_id.to_string(), spec, firmware))
}

/// Runs a keymap operation; progress goes out as `keymap-progress` events.
pub(crate) async fn run(app: &AppHandle, op: KeymapOp) -> CmdResult<KeymapResult> {
    let (progress, mut rx) = mpsc::unbounded_channel::<f32>();
    let emitter = app.clone();
    let forward = tauri::async_runtime::spawn(async move {
        while let Some(p) = rx.recv().await {
            let _ = emitter.emit("keymap-progress", p);
        }
    });
    let (reply, answer) = oneshot::channel();
    app.state::<AppState>()
        .commands
        .send(EngineCommand::Keymap {
            op,
            progress,
            reply,
        })
        .await
        .map_err(|_| "engine stopped".to_string())?;
    let result = answer.await.map_err(|_| "engine stopped".to_string())?;
    let _ = forward.await;
    result
}

pub(crate) async fn read(app: &AppHandle) -> CmdResult<Keymap> {
    match run(app, KeymapOp::Read).await? {
        KeymapResult::Read(keymap) => Ok(keymap),
        KeymapResult::Restored(_) => Err("unexpected answer".into()),
    }
}

#[derive(Serialize)]
pub struct BackupSaved {
    path: String,
    layers: usize,
    assigned_keys: usize,
}

/// Asks where to save, reads the keymap and writes the backup file.
/// `None`: the dialog was cancelled.
#[tauri::command]
pub async fn keymap_backup(app: AppHandle) -> CmdResult<Option<BackupSaved>> {
    let (model, spec, firmware) = connected(&app)?;
    let name = format!("nd104-keymap-{}.json", jiff::Zoned::now().date());
    let dialog = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog
            .dialog()
            .file()
            .set_file_name(&name)
            .add_filter("nd104-sync keymap backup", &["json"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };

    let keymap = read(&app).await?;
    let backup = KeymapBackup::new(keymap, &spec, &model, &firmware);
    let json = backup.to_json().map_err(|e| format!("{e:#}"))?;
    std::fs::write(&path, json).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    tracing::info!("keymap backup saved to {}", path.display());
    Ok(Some(BackupSaved {
        path: path.display().to_string(),
        layers: backup.keymap.layers.len(),
        assigned_keys: backup.keymap.assigned_keys(),
    }))
}

#[derive(Serialize)]
pub struct RestoreCheck {
    name: String,
    created: String,
    backup_firmware: String,
    keyboard_firmware: String,
    /// Keys and knob actions that would change (0: already identical).
    keys: usize,
    knob: usize,
}

/// Asks for a backup file, checks it and compares it with the keyboard.
/// Nothing is written yet. `None`: the dialog was cancelled.
#[tauri::command]
pub async fn keymap_restore_check(app: AppHandle) -> CmdResult<Option<RestoreCheck>> {
    let (model, spec, firmware) = connected(&app)?;
    let dialog = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog
            .dialog()
            .file()
            .add_filter("nd104-sync keymap backup", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    // a backup is a few kB; refuse anything unreasonably large
    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if size > 1024 * 1024 {
        return Err("This file is too large to be a keymap backup.".into());
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let backup = KeymapBackup::from_json(&text, &model, &spec).map_err(|e| format!("{e:#}"))?;

    let current = read(&app).await?;
    let changes = backup.keymap.changes_from(&current, &spec);
    let check = RestoreCheck {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        created: backup.created.clone(),
        backup_firmware: backup.keyboard_firmware.clone(),
        keyboard_firmware: firmware,
        keys: changes.keys.len(),
        knob: changes.knob.len(),
    };
    *app.state::<AppState>().keymap_restore.lock().unwrap() = Some(PendingRestore { backup });
    Ok(Some(check))
}

/// Writes the checked backup to the keyboard and verifies it.
#[tauri::command]
pub async fn keymap_restore_apply(app: AppHandle) -> CmdResult<usize> {
    connected(&app)?;
    let pending = app
        .state::<AppState>()
        .keymap_restore
        .lock()
        .unwrap()
        .take()
        .ok_or("Choose a backup file first.")?;
    match run(&app, KeymapOp::Restore(pending.backup.keymap)).await? {
        KeymapResult::Restored(n) => Ok(n),
        KeymapResult::Read(_) => Err("unexpected answer".into()),
    }
}

/// Forgets a checked backup (Cancel).
#[tauri::command]
pub fn keymap_restore_cancel(app: AppHandle) {
    *app.state::<AppState>().keymap_restore.lock().unwrap() = None;
}
