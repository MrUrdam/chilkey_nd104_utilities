//! The app files packed into the setup by build.rs.

use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use zip::ZipArchive;

static PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.zip"));

fn archive() -> Result<ZipArchive<Cursor<&'static [u8]>>> {
    ZipArchive::new(Cursor::new(PAYLOAD)).context("damaged setup (payload)")
}

/// The setup contains the app (false for development builds of the setup).
pub fn available() -> bool {
    archive().is_ok_and(|a| a.index_for_name(crate::system::APP_EXE).is_some())
}

/// Unpacks all files into `dest`, reporting (bytes done, bytes total).
pub fn extract(dest: &Path, mut progress: impl FnMut(u64, u64)) -> Result<()> {
    let mut zip = archive()?;
    let total: u64 = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|f| f.size()))
        .sum();
    let mut done = 0u64;
    let mut buf = vec![0u8; 256 * 1024];
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry
            .enclosed_name()
            .ok_or_else(|| anyhow!("invalid file name in payload"))?;
        let target = dest.join(name);
        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out =
            File::create(&target).with_context(|| format!("cannot write {}", target.display()))?;
        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
            done += n as u64;
            progress(done, total.max(1));
        }
    }
    Ok(())
}
