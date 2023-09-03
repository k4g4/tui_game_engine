use anyhow::Context;
use clap::Parser;

const TITLE: &str = "Snake";
const UI_COLOR: &str = "#000000";
const BG_COLOR: &str = "#439155";
const DEFAULT_FPS: u32 = 5;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    fps: Option<u32>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = game::Config::new(
        TITLE.into(),
        UI_COLOR,
        BG_COLOR,
        cli.fps.unwrap_or(DEFAULT_FPS),
    )
    .context("parsing command line arguments")?;

    game::init(config).await.context("rendering snake game")?;

    Ok(())
}
