use anyhow::Context;
use clap::Parser;

const TITLE: &str = "Snake";
const BG_COLOR: (u8, u8, u8) = (145, 230, 255);
const DEFAULT_FPS: u32 = 5;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    fps: Option<u32>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = game::Config::new(TITLE.into(), BG_COLOR, cli.fps.unwrap_or(DEFAULT_FPS))
        .context("parsing command line arguments")?;

    game::init(config).await.context("rendering snake game")?;

    Ok(())
}
