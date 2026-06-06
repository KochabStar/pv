#[tokio::main]
async fn main() {
    if let Err(error) = pv::cli::run().await {
        eprintln!("pv: {error:#}");
        std::process::exit(1);
    }
}
