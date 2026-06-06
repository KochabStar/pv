use std::process::Command;

use pv::shim::ShimConfig;

fn main() {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => exit_error(format!("unable to resolve shim path: {error}")),
    };
    let config_path = exe.with_extension("shim");
    let config_text = match std::fs::read_to_string(&config_path) {
        Ok(text) => text,
        Err(error) => exit_error(format!(
            "unable to read shim config {}: {error}",
            config_path.display()
        )),
    };
    let config = match ShimConfig::from_toml(&config_text) {
        Ok(config) => config,
        Err(error) => exit_error(format!("{error}")),
    };

    let args: Vec<String> = std::env::args().skip(1).collect();
    let status = match Command::new(&config.target).args(args).status() {
        Ok(status) => status,
        Err(error) => exit_error(format!("unable to launch target: {error}")),
    };
    std::process::exit(status.code().unwrap_or(1));
}

fn exit_error(message: String) -> ! {
    eprintln!("pv-shim: {message}");
    std::process::exit(1);
}
