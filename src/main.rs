use anyhow::Result;
use wanderling_service::run;

#[tokio::main]
async fn main() -> Result<()> {
    Ok(run().await?)
}
