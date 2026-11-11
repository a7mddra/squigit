use flate2::{write::GzEncoder, Compression};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../..")
        .canonicalize()
        .unwrap();
    let target = std::env::var("TARGET").unwrap();
    let runtime = root.join("binaries").join(format!("paddle-ocr-{target}"));
    let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let source = output.join("runtime.rs");
    let executable = runtime.join(if target.contains("windows") {
        "squigit-ocr.exe"
    } else {
        "squigit-ocr"
    });
    println!("cargo:rerun-if-changed={}", runtime.display());
    println!(
        "cargo:rerun-if-changed={}",
        root.join("native/ocr/VERSION").display()
    );

    if !executable.is_file() {
        if std::env::var("PROFILE").unwrap() == "release" {
            panic!("OCR runtime is missing; run cargo xtask build --ocr before building Squigit");
        }
        println!("cargo:warning=OCR runtime is missing; run cargo xtask build --ocr to embed it");
        fs::write(
            source,
            "const RUNTIME: &[u8] = &[];\nconst RUNTIME_ID: &str = \"unbuilt\";\n",
        )
        .unwrap();
        return;
    }

    let archive_path = output.join("ocr-runtime.tar.gz");
    let encoder = GzEncoder::new(File::create(&archive_path).unwrap(), Compression::fast());
    let mut archive = tar::Builder::new(encoder);
    archive.mode(tar::HeaderMode::Deterministic);
    archive.follow_symlinks(false);
    archive.append_dir_all(".", &runtime).unwrap();
    archive.into_inner().unwrap().finish().unwrap();

    let mut hash = Sha256::new();
    let mut input = BufReader::new(File::open(&archive_path).unwrap());
    let mut buffer = [0; 64 * 1024];
    loop {
        let length = input.read(&mut buffer).unwrap();
        if length == 0 {
            break;
        }
        hash.update(&buffer[..length]);
    }
    let identity = format!("{hash:x}", hash = hash.finalize());
    fs::write(
        source,
        format!(
            "const RUNTIME: &[u8] = include_bytes!({path:?});\nconst RUNTIME_ID: &str = {identity:?};\n",
            path = archive_path,
        ),
    )
    .unwrap();
}
