use bh::cli::Cli;

fn main() {
    if let Err(err) = Cli::run() {
        eprintln!("{:?}", err);
    }
}
