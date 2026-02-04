use clap::Parser;
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = kcap::cli::Args::parse();
    if let Err(err) = kcap::run(args) {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
