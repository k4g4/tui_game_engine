use anyhow::Context;
use clap::Parser;

const TITLE: &str = "Snake";
const DEFAULT_FPS: u32 = 2;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    fps: Option<u32>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = game::Config::new(TITLE.into(), cli.fps.unwrap_or(DEFAULT_FPS))
        .context("parsing command line arguments")?;

    game::render(config).await.context("rendering snake game")?;

    Ok(())
}
