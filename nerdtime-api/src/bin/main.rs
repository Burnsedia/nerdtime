// SPDX-License-Identifier: AGPL-3.0-only
use loco_rs::cli;
use migration::Migrator;
use nerdtime_api::app::App;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    cli::main::<App, Migrator>().await
}
