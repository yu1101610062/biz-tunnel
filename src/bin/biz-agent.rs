use biz_tunnel::{cli, config::Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    cli::run(Role::Agent).await
}
