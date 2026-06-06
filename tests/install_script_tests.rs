#[cfg(windows)]
use std::io::Write;
#[cfg(windows)]
use std::process::Command;

#[test]
fn install_script_bootstraps_release_path_and_main_bucket() {
    let script = std::fs::read_to_string("scripts/install.ps1").expect("read install script");

    assert!(script.contains("param("));
    assert!(script.contains("[string] $Repo"));
    assert!(script.contains("pv.exe"));
    assert!(script.contains("pv-shim.exe"));
    assert!(script.contains("Expand-Archive"));
    assert!(script.contains("buckets\\main"));
    assert!(script.contains("main-bucket"));
    assert!(script.contains("Set-UserPath"));
}

#[test]
fn install_script_updates_path_without_void_array_reverse() {
    let script = std::fs::read_to_string("scripts/install.ps1").expect("read install script");

    assert!(!script.contains("foreach ($dir in [array]::Reverse"));
    assert!(script.contains("for ($index = $Dirs.Count - 1; $index -ge 0; $index--)"));
}

#[cfg(windows)]
#[test]
fn install_script_installs_from_local_release_zip_without_path_update() {
    let temp = tempfile::tempdir().expect("tempdir");
    let release_zip = temp.path().join("pv-release.zip");
    write_release_zip(&release_zip);
    let install_dir = temp.path().join("home");

    let output = Command::new(powershell_exe())
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "scripts/install.ps1",
            "-DownloadUrl",
        ])
        .arg(&release_zip)
        .arg("-InstallDir")
        .arg(&install_dir)
        .arg("-NoPathUpdate")
        .output()
        .expect("run install script");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(install_dir.join("bin").join("pv.exe").exists());
    assert!(install_dir.join("bin").join("pv-shim.exe").exists());
    assert!(install_dir
        .join("buckets")
        .join("main")
        .join("git.toml")
        .exists());
    assert!(install_dir.join("config.toml").exists());
}

#[cfg(windows)]
fn write_release_zip(path: &std::path::Path) {
    let file = std::fs::File::create(path).expect("release zip");
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    zip.start_file("pv.exe", options).expect("pv.exe entry");
    zip.write_all(b"fake pv").expect("write pv.exe");
    zip.start_file("pv-shim.exe", options)
        .expect("pv-shim.exe entry");
    zip.write_all(b"fake shim").expect("write pv-shim.exe");
    zip.start_file("buckets/main/git.toml", options)
        .expect("bucket entry");
    zip.write_all(b"name = 'git'").expect("write bucket");
    zip.finish().expect("finish release zip");
}

#[cfg(windows)]
fn powershell_exe() -> &'static str {
    if Command::new("pwsh")
        .arg("-NoProfile")
        .arg("-Command")
        .arg("$PSVersionTable.PSVersion.ToString()")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
    {
        "pwsh"
    } else {
        "powershell"
    }
}
