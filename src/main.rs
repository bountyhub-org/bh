use bh::cli::Cli;

fn main() {
    if let Err(err) = Cli::run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
