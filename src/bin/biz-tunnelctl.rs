use biz_tunnel::cli;

#[tokio::main]
async fn main() {
    if let Err(error) = cli::run_ctl().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
