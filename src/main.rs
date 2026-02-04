use clap::Parser;
use tracing_subscriber::EnvFilter;

fn main() {
    // 初始化结构化日志，便于排查 CLI 执行问题。
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = kcap::cli::Args::parse();
    // 遇到错误时快速失败并给出可读的错误信息。
    if let Err(err) = kcap::run(args) {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
