//! The SignalRGB plugin for the ND104 (`signalrgb/Chilkey_ND104.js`): built
//! into the app and copied into SignalRGB's user plugin folder
//! (`Documents\WhirlwindFX\Plugins`), where it survives SignalRGB updates.
//! SignalRGB then colours every key over the keyboard's lighting channel
//! (docs/protocol.md § 5.4).

use std::path::PathBuf;

use serde::Serialize;

type CmdResult<T> = Result<T, String>;

const PLUGIN: &str = include_str!("../../signalrgb/Chilkey_ND104.js");
const FILE_NAME: &str = "Chilkey_ND104.js";

#[derive(Debug, Clone, Serialize)]
pub struct SignalRgbStatus {
    /// SignalRGB's folder in Documents exists (it is created by SignalRGB).
    pub signalrgb_found: bool,
    /// The plugin file is there …
    pub installed: bool,
    /// … and is the one of this app version.
    pub current: bool,
    /// Where the plugin goes.
    pub folder: String,
}

/// `Documents\WhirlwindFX` – Documents as Windows knows it (also when
/// OneDrive moved it).
fn whirlwind_dir() -> Option<PathBuf> {
    Some(
        directories::UserDirs::new()?
            .document_dir()?
            .join("WhirlwindFX"),
    )
}

fn status() -> CmdResult<SignalRgbStatus> {
    let base = whirlwind_dir().ok_or("Documents folder not found")?;
    let folder = base.join("Plugins");
    let file = folder.join(FILE_NAME);
    let content = std::fs::read_to_string(&file).ok();
    Ok(SignalRgbStatus {
        signalrgb_found: base.is_dir(),
        installed: content.is_some(),
        current: content.as_deref().map(|c| c.replace("\r\n", "\n"))
            == Some(PLUGIN.replace("\r\n", "\n")),
        folder: folder.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
pub fn signalrgb_status() -> CmdResult<SignalRgbStatus> {
    status()
}

/// Installs or updates the plugin.
#[tauri::command]
pub fn signalrgb_install() -> CmdResult<SignalRgbStatus> {
    let folder = whirlwind_dir()
        .ok_or("Documents folder not found")?
        .join("Plugins");
    std::fs::create_dir_all(&folder)
        .map_err(|e| format!("cannot create {}: {e}", folder.display()))?;
    let file = folder.join(FILE_NAME);
    std::fs::write(&file, PLUGIN).map_err(|e| format!("cannot write {}: {e}", file.display()))?;
    tracing::info!("SignalRGB plugin installed: {}", file.display());
    status()
}

#[tauri::command]
pub fn signalrgb_remove() -> CmdResult<SignalRgbStatus> {
    let file = whirlwind_dir()
        .ok_or("Documents folder not found")?
        .join("Plugins")
        .join(FILE_NAME);
    if file.exists() {
        std::fs::remove_file(&file)
            .map_err(|e| format!("cannot remove {}: {e}", file.display()))?;
        tracing::info!("SignalRGB plugin removed: {}", file.display());
    }
    status()
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_plugin_targets_the_lighting_channel() {
        let p = super::PLUGIN;
        assert!(p.contains("return 0x6D67;") && p.contains("return 0x016C;"));
        assert!(p.contains("endpoint.interface === 4 && endpoint.usage_page === 0xFF59"));
        assert!(p.contains("const REPORT_ID = 0x05;"));
    }
}
