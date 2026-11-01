//! Windows integration: folders, "Apps" entry, autostart, shortcuts,
//! running app instances, WebView2.

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use windows::core::{Interface, GUID, HSTRING, PWSTR};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, IPersistFile, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
};
use windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore;
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_LocalAppData, FOLDERID_Programs, IShellLinkW, SHGetKnownFolderPath,
    ShellLink, KF_FLAG_DEFAULT,
};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, IDYES, MB_ICONQUESTION, MB_YESNO};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
use winreg::RegKey;

/// Name shown in Windows: "Apps", shortcuts, autostart entry (the app's
/// autostart plugin uses its product name too).
pub const APP_NAME: &str = "ND104 Studio";
/// Technical name: folders and files.
pub const APP_ID: &str = "nd104-studio";
pub const APP_EXE: &str = "nd104-studio.exe";
pub const SETUP_EXE: &str = "nd104-studio-setup.exe";
/// Same id as the app (tauri.conf.json): Windows notifications are bound to it.
pub const APP_USER_MODEL_ID: &str = "io.github.mrurdam.nd104-studio";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\nd104-studio";

/// The app's former name (up to 1.5.x): used for folders, files, the "Apps"
/// entry, the autostart entry and the shortcuts alike.
pub const OLD_ID: &str = "nd104-sync";
pub const OLD_APP_EXE: &str = "nd104-sync.exe";
const OLD_SETUP_EXE: &str = "nd104-sync-setup.exe";
const OLD_UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\nd104-sync";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const PUBLISHER: &str = "MrUrdam";
const PROJECT_URL: &str = "https://github.com/MrUrdam/chilkey_nd104_utilities";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// ---------------------------------------------------------------------------
// Folders

fn known_folder(id: &GUID) -> Result<PathBuf> {
    unsafe {
        let p: PWSTR = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None)?;
        let path = PathBuf::from(p.to_string()?);
        CoTaskMemFree(Some(p.0 as _));
        Ok(path)
    }
}

/// `%LOCALAPPDATA%\Programs\nd104-studio` – where this setup installs.
pub fn install_dir() -> Result<PathBuf> {
    Ok(known_folder(&FOLDERID_LocalAppData)?
        .join("Programs")
        .join(APP_ID))
}

/// `%LOCALAPPDATA%\nd104-studio` – logs; also where the old NSIS setup installed.
pub fn data_dir() -> Result<PathBuf> {
    Ok(known_folder(&FOLDERID_LocalAppData)?.join(APP_ID))
}

fn start_menu_link() -> Result<PathBuf> {
    Ok(known_folder(&FOLDERID_Programs)?.join(format!("{APP_NAME}.lnk")))
}

fn desktop_link() -> Result<PathBuf> {
    Ok(known_folder(&FOLDERID_Desktop)?.join(format!("{APP_NAME}.lnk")))
}

fn old_links() -> Result<[PathBuf; 2]> {
    Ok([
        known_folder(&FOLDERID_Programs)?.join(format!("{OLD_ID}.lnk")),
        known_folder(&FOLDERID_Desktop)?.join(format!("{OLD_ID}.lnk")),
    ])
}

// ---------------------------------------------------------------------------
// "Apps" entry (uninstall registry key)

#[derive(Debug, Clone)]
pub struct Installed {
    pub version: String,
    pub location: PathBuf,
    /// Installed by the old NSIS setup (≤ 1.0.1).
    pub legacy: bool,
    /// Installed under the former name nd104-sync (≤ 1.5.x).
    pub old_name: bool,
}

/// The installation, also one under the former name nd104-sync.
pub fn installed() -> Option<Installed> {
    read_installed(UNINSTALL_KEY, SETUP_EXE, false).or_else(old_installed)
}

/// An installation under the former name nd104-sync.
pub fn old_installed() -> Option<Installed> {
    read_installed(OLD_UNINSTALL_KEY, OLD_SETUP_EXE, true)
}

fn read_installed(key: &str, setup_exe: &str, old_name: bool) -> Option<Installed> {
    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey(key).ok()?;
    let version: String = key.get_value("DisplayVersion").ok()?;
    let location: String = key.get_value("InstallLocation").unwrap_or_default();
    let uninstall: String = key.get_value("UninstallString").unwrap_or_default();
    Some(Installed {
        version,
        location: PathBuf::from(location.trim().trim_matches('"')),
        legacy: !uninstall.to_lowercase().contains(setup_exe),
        old_name,
    })
}

/// Removes the Windows entries of the former name: "Apps" entry, autostart
/// entry and shortcuts. The new ones are created by the install.
pub fn remove_old_integration() -> Result<()> {
    for link in old_links()? {
        if link.exists() {
            std::fs::remove_file(&link)
                .with_context(|| format!("cannot remove {}", link.display()))?;
        }
    }
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(run) = hkcu.open_subkey_with_flags(RUN_KEY, winreg::enums::KEY_SET_VALUE) {
        let _ = run.delete_value(OLD_ID);
    }
    match hkcu.delete_subkey_all(OLD_UNINSTALL_KEY) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// nd104-studio installed for all users via the MSI?
pub fn msi_installed() -> bool {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    [
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ]
    .iter()
    .filter_map(|path| {
        hklm.open_subkey_with_flags(path, KEY_READ | KEY_WOW64_64KEY)
            .ok()
    })
    .any(|root| {
        root.enum_keys().flatten().any(|name| {
            root.open_subkey(&name).is_ok_and(|k| {
                k.get_value::<String, _>("DisplayName")
                    .is_ok_and(|n| n == APP_NAME || n == OLD_ID)
                    && k.get_value::<u32, _>("WindowsInstaller")
                        .is_ok_and(|v| v == 1)
            })
        })
    })
}

pub fn register(dir: &Path, version: &str) -> Result<()> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(UNINSTALL_KEY)?;
    let setup = dir.join(SETUP_EXE);
    let size_kb = u32::try_from(dir_size(dir) / 1024).unwrap_or(u32::MAX);
    key.set_value("DisplayName", &APP_NAME)?;
    key.set_value("DisplayVersion", &version)?;
    key.set_value("Publisher", &PUBLISHER)?;
    key.set_value("DisplayIcon", &dir.join(APP_EXE).display().to_string())?;
    key.set_value("InstallLocation", &dir.display().to_string())?;
    key.set_value(
        "UninstallString",
        &format!("\"{}\" --uninstall", setup.display()),
    )?;
    key.set_value(
        "QuietUninstallString",
        &format!("\"{}\" --uninstall --silent", setup.display()),
    )?;
    key.set_value("URLInfoAbout", &PROJECT_URL)?;
    key.set_value("HelpLink", &PROJECT_URL)?;
    key.set_value("NoModify", &1u32)?;
    key.set_value("NoRepair", &1u32)?;
    key.set_value("EstimatedSize", &size_kb)?;
    Ok(())
}

pub fn unregister() -> Result<()> {
    match RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(UNINSTALL_KEY) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn dir_size(dir: &Path) -> u64 {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| match e.metadata() {
                    Ok(m) if m.is_dir() => dir_size(&e.path()),
                    Ok(m) => m.len(),
                    Err(_) => 0,
                })
                .sum()
        })
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Autostart (same entry the app's autostart plugin writes)

/// Also true for the entry of the former name, so an update keeps the choice.
pub fn autostart_enabled() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .is_ok_and(|k| {
            k.get_value::<String, _>(APP_NAME).is_ok() || k.get_value::<String, _>(OLD_ID).is_ok()
        })
}

pub fn set_autostart(exe: Option<&Path>) -> Result<()> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY)?;
    match exe {
        Some(exe) => key.set_value(APP_NAME, &format!("\"{}\" --minimized", exe.display()))?,
        None => match key.delete_value(APP_NAME) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        },
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Shortcuts

/// Also true for the shortcut of the former name, so an update keeps it.
pub fn has_desktop_shortcut() -> bool {
    desktop_link().is_ok_and(|p| p.exists())
        || old_links().is_ok_and(|[_, desktop]| desktop.exists())
}

pub fn create_shortcuts(exe: &Path, desktop: bool) -> Result<()> {
    create_link(&start_menu_link()?, exe)?;
    let desktop_lnk = desktop_link()?;
    if desktop {
        create_link(&desktop_lnk, exe)?;
    } else if desktop_lnk.exists() {
        std::fs::remove_file(&desktop_lnk)?;
    }
    Ok(())
}

pub fn remove_shortcuts() -> Result<()> {
    for link in [start_menu_link()?, desktop_link()?] {
        if link.exists() {
            std::fs::remove_file(&link)
                .with_context(|| format!("cannot remove {}", link.display()))?;
        }
    }
    Ok(())
}

/// `.lnk` with the app's AppUserModelID, so Windows shows its notifications.
fn create_link(link: &Path, exe: &Path) -> Result<()> {
    if let Some(dir) = link.parent() {
        std::fs::create_dir_all(dir)?;
    }
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let shell: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        shell.SetPath(&HSTRING::from(exe.as_os_str()))?;
        if let Some(dir) = exe.parent() {
            shell.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()))?;
        }
        shell.SetDescription(&HSTRING::from(
            "Display, lighting and keymap for the Chilkey ND104",
        ))?;
        shell.SetIconLocation(&HSTRING::from(exe.as_os_str()), 0)?;
        let store: IPropertyStore = shell.cast()?;
        store.SetValue(&PKEY_AppUserModel_ID, &PROPVARIANT::from(APP_USER_MODEL_ID))?;
        store.Commit()?;
        let file: IPersistFile = shell.cast()?;
        file.Save(&HSTRING::from(link.as_os_str()), true)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Running instances

fn running(dirs: &[PathBuf]) -> Vec<u32> {
    let wanted: Vec<String> = dirs
        .iter()
        .map(|d| d.display().to_string().to_lowercase())
        .collect();
    let mut pids = Vec::new();
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return pids;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut ok = Process32FirstW(snapshot, &mut entry).is_ok();
        while ok {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..len]);
            if name.eq_ignore_ascii_case(APP_EXE) || name.eq_ignore_ascii_case(OLD_APP_EXE) {
                if let Some(path) = process_path(entry.th32ProcessID) {
                    let dir = path
                        .parent()
                        .map(|d| d.display().to_string().to_lowercase());
                    if dir.is_some_and(|d| wanted.contains(&d)) {
                        pids.push(entry.th32ProcessID);
                    }
                }
            }
            ok = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
    }
    pids
}

fn process_path(pid: u32) -> Option<PathBuf> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        result.ok()?;
        Some(PathBuf::from(String::from_utf16_lossy(
            &buf[..len as usize],
        )))
    }
}

/// Stops nd104-studio running from one of `dirs`: first asks it to quit
/// (`--quit`, understood since 1.1.0), then ends it.
pub fn stop_app(dirs: &[PathBuf], graceful_exe: Option<&Path>) -> Result<()> {
    if running(dirs).is_empty() {
        return Ok(());
    }
    if let Some(exe) = graceful_exe.filter(|e| e.exists()) {
        let _ = Command::new(exe)
            .arg("--quit")
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
        let deadline = Instant::now() + Duration::from_secs(6);
        while Instant::now() < deadline && !running(dirs).is_empty() {
            std::thread::sleep(Duration::from_millis(250));
        }
    }
    for pid in running(dirs) {
        unsafe {
            if let Ok(handle) = OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, false, pid) {
                let _ = TerminateProcess(handle, 0);
                WaitForSingleObject(handle, 5000);
                let _ = CloseHandle(handle);
            }
        }
    }
    if running(dirs).is_empty() {
        Ok(())
    } else {
        Err(anyhow!("{APP_NAME} could not be closed"))
    }
}

pub fn launch(exe: &Path) -> Result<()> {
    Command::new(exe)
        .current_dir(exe.parent().unwrap_or(Path::new(".")))
        .spawn()
        .with_context(|| format!("cannot start {}", exe.display()))?;
    Ok(())
}

/// Runs the old NSIS uninstaller silently and waits for it (`_?=` keeps it
/// from copying itself away).
pub fn run_legacy_uninstaller(dir: &Path) -> Result<()> {
    let uninstaller = dir.join("uninstall.exe");
    if !uninstaller.exists() {
        return Ok(());
    }
    let status = Command::new(&uninstaller)
        .arg("/S")
        .arg(format!("_?={}", dir.display()))
        .status()
        .context("cannot run the old uninstaller")?;
    if !status.success() {
        return Err(anyhow!("old uninstaller failed ({status})"));
    }
    let _ = std::fs::remove_file(&uninstaller);
    Ok(())
}

// ---------------------------------------------------------------------------
// Uninstall runs from a temporary copy (the setup in the install folder
// cannot delete itself).

pub fn relaunch_from_temp(args: &[String]) -> Result<std::process::Child> {
    let me = std::env::current_exe()?;
    let copy = std::env::temp_dir().join(format!("nd104-studio-setup-{}.exe", std::process::id()));
    std::fs::copy(&me, &copy)?;
    Ok(Command::new(&copy).args(args).arg("--relaunched").spawn()?)
}

/// This setup is the copy inside the installation folder.
pub fn running_from_install_dir() -> bool {
    let (Ok(me), Ok(dir)) = (std::env::current_exe(), install_dir()) else {
        return false;
    };
    me.parent().is_some_and(|p| {
        p.to_string_lossy()
            .eq_ignore_ascii_case(&dir.to_string_lossy())
    })
}

/// Deletes this (temporary) executable shortly after it has exited.
pub fn delete_self_later() {
    if let Ok(me) = std::env::current_exe() {
        let _ = Command::new("cmd")
            .args([
                "/C",
                "ping",
                "127.0.0.1",
                "-n",
                "3",
                ">",
                "nul",
                "&",
                "del",
                "/f",
                "/q",
            ])
            .arg(&me)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
}

// ---------------------------------------------------------------------------
// WebView2 (needed by the app and by this setup's window)

const WEBVIEW2_CLIENT: &str =
    r"Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
const WEBVIEW2_BOOTSTRAPPER: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";

pub fn webview2_installed() -> bool {
    let check = |root: RegKey, path: String| {
        root.open_subkey_with_flags(path, KEY_READ)
            .and_then(|k| k.get_value::<String, _>("pv"))
            .is_ok_and(|v| !v.is_empty() && v != "0.0.0.0")
    };
    check(
        RegKey::predef(HKEY_LOCAL_MACHINE),
        format!(r"SOFTWARE\WOW6432Node\{WEBVIEW2_CLIENT}"),
    ) || check(
        RegKey::predef(HKEY_LOCAL_MACHINE),
        format!(r"SOFTWARE\{WEBVIEW2_CLIENT}"),
    ) || check(
        RegKey::predef(HKEY_CURRENT_USER),
        format!(r"Software\{WEBVIEW2_CLIENT}"),
    )
}

pub fn ask(title: &str, text: &str) -> bool {
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(text),
            &HSTRING::from(title),
            MB_YESNO | MB_ICONQUESTION,
        ) == IDYES
    }
}

/// Downloads and runs Microsoft's WebView2 bootstrapper silently.
pub fn install_webview2() -> Result<()> {
    let target = std::env::temp_dir().join("MicrosoftEdgeWebview2Setup.exe");
    let bytes = tauri::async_runtime::block_on(async {
        reqwest::get(WEBVIEW2_BOOTSTRAPPER)
            .await?
            .error_for_status()?
            .bytes()
            .await
    })
    .context("cannot download the WebView2 runtime")?;
    std::fs::write(&target, &bytes)?;
    let status = Command::new(&target)
        .args(["/silent", "/install"])
        .status()?;
    let _ = std::fs::remove_file(&target);
    if !status.success() || !webview2_installed() {
        return Err(anyhow!("WebView2 installation failed"));
    }
    Ok(())
}
