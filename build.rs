use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=windows-app.manifest");

    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.ends_with("pc-windows-msvc") {
        return;
    }

    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("windows-app.manifest")
        .canonicalize()
        .expect("windows-app.manifest exists");
    let manifest_arg = format!("/MANIFESTINPUT:{}", manifest.display());

    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bins={manifest_arg}");
    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-tests={manifest_arg}");
}
