use std::fs;
use std::thread;

use pv::engine::download::{download_to_cache, verify_sha256};
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
    )
    .await
    .expect("download");

    handle.join().expect("server thread");
    assert_eq!(fs::read(file).expect("read file"), b"hello");
}
