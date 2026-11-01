//! Commands for the keyboard lighting, the keyboard settings and the own
//! picture/GIF on the screen.
//! Both go through the engine, so they never run at the same time as a
//! time/weather sync.

use std::path::PathBuf;

use base64::Engine as _;
use nd104_core::engine::EngineCommand;
use nd104_core::keyboard_settings::{KeyboardSetting, KeyboardSettings, SettingsOp};
use nd104_core::lighting::{LightChange, LightOp, LightZone, ZoneState};
use nd104_core::model::{self, PictureSpec};
use nd104_core::picture::{self, Fit, PictureNote, PictureProblem, PreparedPicture};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::{mpsc, oneshot};

use crate::AppState;

type CmdResult<T> = Result<T, String>;

/// The picked file, kept until it is sent or replaced.
pub(crate) struct PendingPicture {
    name: String,
    bytes: Vec<u8>,
    prepared: Option<PreparedPicture>,
}

async fn engine<T>(
    app: &AppHandle,
    make: impl FnOnce(oneshot::Sender<T>) -> EngineCommand,
) -> CmdResult<T> {
    let (reply, rx) = oneshot::channel();
    app.state::<AppState>()
        .commands
        .send(make(reply))
        .await
        .map_err(|_| "engine stopped".to_string())?;
    rx.await.map_err(|_| "engine stopped".to_string())
}

#[tauri::command]
pub async fn light_read(app: AppHandle, zone: LightZone) -> CmdResult<ZoneState> {
    let state = engine(&app, |r| EngineCommand::Light(LightOp::Read(zone), r)).await??;
    Ok(state.unwrap_or_default())
}

#[tauri::command]
pub async fn light_apply(
    app: AppHandle,
    zone: LightZone,
    changes: Vec<LightChange>,
    save: bool,
) -> CmdResult<()> {
    engine(&app, |r| {
        EngineCommand::Light(
            LightOp::Apply {
                zone,
                changes,
                save,
            },
            r,
        )
    })
    .await??;
    Ok(())
}

#[tauri::command]
pub async fn keyboard_settings_read(app: AppHandle) -> CmdResult<KeyboardSettings> {
    engine(&app, |r| EngineCommand::Settings(SettingsOp::Read, r)).await?
}

/// Writes and stores one setting; returns all settings read back afterwards.
#[tauri::command]
pub async fn keyboard_settings_write(
    app: AppHandle,
    setting: KeyboardSetting,
    value: u8,
) -> CmdResult<KeyboardSettings> {
    engine(&app, |r| {
        EngineCommand::Settings(SettingsOp::Write(setting, value), r)
    })
    .await?
}

/// Limits of the connected (or else the default) model.
fn spec(app: &AppHandle) -> Option<PictureSpec> {
    let status = app.state::<AppState>().status.borrow().clone();
    let models = model::registry();
    let model = match &status.device {
        nd104_core::device::DeviceState::Ready(d) => models.iter().find(|m| m.id() == d.model_id),
        _ => None,
    }
    .or_else(|| models.first());
    model.and_then(|m| m.picture_spec())
}

#[derive(Serialize)]
pub struct PictureCheck {
    name: String,
    file_bytes: usize,
    spec: PictureSpec,
    /// Why the file cannot be used; the rest is empty then.
    problem: Option<PictureProblem>,
    format: Option<&'static str>,
    width: u32,
    height: u32,
    frames: usize,
    payload_bytes: usize,
    notes: Vec<PictureNote>,
    /// The frames exactly as they are sent (JPEG data URLs), for the preview.
    preview_frames: Vec<String>,
    /// Display time per frame in ms.
    delays_ms: Vec<u32>,
}

fn check(pending: &mut PendingPicture, spec: PictureSpec, fit: Fit) -> PictureCheck {
    let mut result = PictureCheck {
        name: pending.name.clone(),
        file_bytes: pending.bytes.len(),
        spec,
        problem: None,
        format: None,
        width: 0,
        height: 0,
        frames: 0,
        payload_bytes: 0,
        notes: Vec::new(),
        preview_frames: Vec::new(),
        delays_ms: Vec::new(),
    };
    match picture::prepare(&pending.bytes, &spec, fit) {
        Ok(p) => {
            result.format = Some(p.format);
            result.width = p.source_width;
            result.height = p.source_height;
            result.frames = p.frames.len();
            result.payload_bytes = p.payload_bytes;
            result.notes = p.notes.clone();
            result.preview_frames = p
                .frames
                .iter()
                .map(|f| {
                    format!(
                        "data:image/jpeg;base64,{}",
                        base64::engine::general_purpose::STANDARD.encode(&f.jpeg)
                    )
                })
                .collect();
            result.delays_ms = p.frames.iter().map(|f| u32::from(f.delay) * 10).collect();
            pending.prepared = Some(p);
        }
        Err(problem) => {
            result.problem = Some(problem);
            pending.prepared = None;
        }
    }
    result
}

/// Opens a file dialog and checks the picked picture. `None`: cancelled.
#[tauri::command]
pub async fn picture_pick(app: AppHandle, fit: Fit) -> CmdResult<Option<PictureCheck>> {
    let spec = spec(&app).ok_or("this keyboard does not support own pictures")?;
    let dialog = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog
            .dialog()
            .file()
            .add_filter("PNG, JPEG, GIF, BMP", &["png", "jpg", "jpeg", "gif", "bmp"])
            .blocking_pick_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else {
        return Ok(None);
    };
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let bytes = read_limited(&path, spec.max_file_bytes)?;
    let mut pending = PendingPicture {
        name,
        bytes,
        prepared: None,
    };
    let result = tauri::async_runtime::spawn_blocking(move || {
        let r = check(&mut pending, spec, fit);
        (r, pending)
    })
    .await
    .map_err(|e| e.to_string())?;
    *app.state::<AppState>().picture.lock().unwrap() = Some(result.1);
    Ok(Some(result.0))
}

/// Reads at most `max + 1` bytes, so a huge file is not loaded completely
/// (the check then reports it as too large).
fn read_limited(path: &PathBuf, max: usize) -> CmdResult<Vec<u8>> {
    use std::io::Read;
    let file =
        std::fs::File::open(path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    Ok(bytes)
}

/// Converts the picked picture again with another fit.
#[tauri::command]
pub async fn picture_refit(app: AppHandle, fit: Fit) -> CmdResult<PictureCheck> {
    let spec = spec(&app).ok_or("this keyboard does not support own pictures")?;
    let pending = app
        .state::<AppState>()
        .picture
        .lock()
        .unwrap()
        .take()
        .ok_or("no picture selected")?;
    let (result, pending) = tauri::async_runtime::spawn_blocking(move || {
        let mut pending = pending;
        let r = check(&mut pending, spec, fit);
        (r, pending)
    })
    .await
    .map_err(|e| e.to_string())?;
    *app.state::<AppState>().picture.lock().unwrap() = Some(pending);
    Ok(result)
}

/// Sends the checked picture to the screen; progress as `picture-progress`
/// events (0.0 … 1.0).
#[tauri::command]
pub async fn picture_upload(app: AppHandle) -> CmdResult<()> {
    let frames = app
        .state::<AppState>()
        .picture
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|p| p.prepared.as_ref())
        .map(|p| p.frames.clone())
        .ok_or("no checked picture to send")?;
    let (progress, mut rx) = mpsc::unbounded_channel::<f32>();
    let emitter = app.clone();
    let forward = tauri::async_runtime::spawn(async move {
        while let Some(p) = rx.recv().await {
            let _ = emitter.emit("picture-progress", p);
        }
    });
    let result = engine(&app, |reply| EngineCommand::UploadPicture {
        frames,
        progress,
        reply,
    })
    .await?;
    let _ = forward.await;
    result
}
