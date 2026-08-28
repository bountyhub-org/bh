use bh::cli::Cli;

#[tokio::main]
async fn main() {
    if let Err(err) = Cli::run().await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
