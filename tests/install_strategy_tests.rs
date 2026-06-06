use std::fs;
use std::io::Write;

use pv::engine::install::{
    archive::ArchiveInstallStrategy, single::SingleInstallStrategy, InstallStrategy,
};
use pv::manifest::{Architecture, InstallType, Manifest};
use tempfile::tempdir;
use zip::write::SimpleFileOptions;

fn manifest(install_type: InstallType) -> Manifest {
    Manifest {
        name: "tool".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        homepage: None,
        license: None,
        install_type,
        installer: None,
        bin: vec!["tool.exe".to_string()],
        architecture: Architecture {
            x64: pv::manifest::ArchitectureManifest {
                url: "file://payload".to_string(),
                hash: "sha256:hash".to_string(),
                extract_dir: None,
                bin: Vec::new(),
            },
        },
        env: Default::default(),
    }
}

#[test]
fn single_strategy_copies_file_to_version_dir() {
    let dir = tempdir().expect("tempdir");
    let source = dir.path().join("tool.exe");
    let target = dir.path().join("install");
    fs::write(&source, b"tool").expect("source");

    SingleInstallStrategy
        .install(&manifest(InstallType::Single), &source, &target)
        .expect("install");

    assert_eq!(
        fs::read(target.join("tool.exe")).expect("installed"),
        b"tool"
    );
}

#[test]
fn archive_strategy_extracts_zip_payload() {
    let dir = tempdir().expect("tempdir");
    let archive = dir.path().join("tool.zip");
    let file = fs::File::create(&archive).expect("archive");
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("tool.exe", SimpleFileOptions::default())
        .expect("start file");
    zip.write_all(b"tool").expect("write zip");
    zip.finish().expect("finish zip");
    let target = dir.path().join("install");

    ArchiveInstallStrategy
        .install(&manifest(InstallType::Archive), &archive, &target)
        .expect("install");

    assert_eq!(
        fs::read(target.join("tool.exe")).expect("installed"),
        b"tool"
    );
}
