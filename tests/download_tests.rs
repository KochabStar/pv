use std::fs;
use std::path::Path;
use std::thread;

use pv::config::DownloadConfig;
use pv::engine::download::{build_aria2_command, download_to_cache, verify_sha256};
use tempfile::tempdir;
use tiny_http::{Response, Server};

#[test]
fn verifies_sha256_hash() {
    let dir = tempdir().expect("tempdir");
    let file = dir.path().join("payload.txt");
    fs::write(&file, b"hello").expect("write payload");

    verify_sha256(
        &file,
        "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    )
    .expect("checksum");

    assert!(verify_sha256(
        &file,
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    )
    .is_err());
}

#[tokio::test]
async fn downloads_from_local_http_server_and_checks_hash() {
    let server = Server::http("127.0.0.1:0").expect("server");
    let address = server.server_addr().to_string();
    let handle = thread::spawn(move || {
        let mut request = server.recv().expect("request");
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body).expect("read");
        request
            .respond(Response::from_string("hello"))
            .expect("respond");
    });
    let dir = tempdir().expect("tempdir");

    let file = download_to_cache(
        &format!("http://{address}/payload.txt"),
        dir.path(),
        "payload.txt",
        "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
        &pv::engine::download::DownloadOptions::from_config(&DownloadConfig::default()),
    )
    .await
    .expect("download");

    handle.join().expect("server thread");
    assert_eq!(fs::read(file).expect("read file"), b"hello");
}

#[tokio::test]
async fn copies_local_file_url_to_cache_and_checks_hash() {
    let source_dir = tempdir().expect("source tempdir");
    let cache_dir = tempdir().expect("cache tempdir");
    let source = source_dir.path().join("payload.txt");
    fs::write(&source, b"hello").expect("write payload");

    let file = download_to_cache(
        &format!("file://{}", source.display()),
        cache_dir.path(),
        "payload.txt",
        "sha256:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
        &pv::engine::download::DownloadOptions::from_config(&DownloadConfig::default()),
    )
    .await
    .expect("download");

    assert_eq!(fs::read(file).expect("read file"), b"hello");
}

#[test]
fn builds_aria2_command_with_scoop_style_options() {
    let options = pv::engine::download::DownloadOptions {
        aria2_enabled: true,
        aria2_split: 8,
        aria2_max_connection_per_server: 4,
        aria2_min_split_size: "10M".to_string(),
    };

    let command = build_aria2_command(
        "https://example.invalid/tool.zip",
        Path::new("C:/cache"),
        "tool.zip",
        &options,
    );

    assert_eq!(command.program, "aria2c");
    assert!(command.args.contains(&"--allow-overwrite=true".to_string()));
    assert!(command
        .args
        .contains(&"--auto-file-renaming=false".to_string()));
    assert!(command.args.contains(&"--continue=true".to_string()));
    assert!(command.args.contains(&"--split=8".to_string()));
    assert!(command
        .args
        .contains(&"--max-connection-per-server=4".to_string()));
    assert!(command.args.contains(&"--min-split-size=10M".to_string()));
    assert!(command
        .args
        .contains(&"https://example.invalid/tool.zip".to_string()));
}
