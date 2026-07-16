mod state;
mod tools;

use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let app_state = state::AppState::new()?;

    let running = app_state.serve(rmcp::transport::stdio()).await?;

    running.waiting().await?;

    Ok(())
}
