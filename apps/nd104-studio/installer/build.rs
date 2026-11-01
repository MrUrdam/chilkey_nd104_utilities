//! Packs the app files into the setup (payload.zip in OUT_DIR).
//!
//! The app must be built first (`npm run tauri build`), so that
//! `target/release/nd104-studio.exe` exists. `ND104_PAYLOAD_DIR` overrides the
//! folder with the executables. Without the app the payload stays empty and
//! the setup refuses to install (development builds of the setup UI).

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

fn main() {
    pack_payload();
    tauri_build::build();
}

fn pack_payload() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let app_root = manifest.parent().unwrap().to_path_buf();
    let repo_root = app_root.parent().unwrap().parent().unwrap().to_path_buf();
    println!("cargo:rerun-if-env-changed=ND104_PAYLOAD_DIR");
    let bin_dir = std::env::var("ND104_PAYLOAD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| app_root.join("target").join("release"));

    // (source, name in the installation folder, required)
    let files: Vec<(PathBuf, &str, bool)> = vec![
        (bin_dir.join("nd104-studio.exe"), "nd104-studio.exe", true),
        (bin_dir.join("nd104-cli.exe"), "nd104-cli.exe", false),
        (bin_dir.join("nd104-probe.exe"), "nd104-probe.exe", false),
        (app_root.join("README.md"), "README.md", false),
        (app_root.join("CHANGELOG.md"), "CHANGELOG.md", false),
        (repo_root.join("LICENSE"), "LICENSE", false),
    ];
    for (source, _, _) in &files {
        println!("cargo:rerun-if-changed={}", source.display());
    }

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("payload.zip");
    let mut zip = zip::ZipWriter::new(File::create(&out).expect("cannot create payload.zip"));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(9));

    if !files[0].0.exists() {
        println!(
            "cargo:warning=nd104-studio.exe not found in {} – the setup is built WITHOUT the app (it will refuse to install). Run `npm run tauri build` first.",
            bin_dir.display()
        );
        zip.finish().expect("cannot finish payload.zip");
        return;
    }
    for (source, name, required) in &files {
        if !source.exists() {
            assert!(!required, "missing {}", source.display());
            println!(
                "cargo:warning=optional payload file missing: {}",
                source.display()
            );
            continue;
        }
        add(&mut zip, source, name, options);
    }
    zip.finish().expect("cannot finish payload.zip");
}

fn add(zip: &mut zip::ZipWriter<File>, source: &Path, name: &str, options: SimpleFileOptions) {
    let data =
        std::fs::read(source).unwrap_or_else(|e| panic!("cannot read {}: {e}", source.display()));
    zip.start_file(name, options).expect("zip entry");
    zip.write_all(&data).expect("zip write");
}
