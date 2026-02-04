use clap::Parser;
use tracing_subscriber::EnvFilter;

fn main() {
    // Initialize structured logging to aid CLI troubleshooting.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = kcap::cli::Args::parse();
    // Fail fast with a readable error message.
    if let Err(err) = kcap::run(args) {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
