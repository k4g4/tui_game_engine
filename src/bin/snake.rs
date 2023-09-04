use anyhow::Context;
use clap::Parser;
use std::fs::File;
use tracing::Level;

const TITLE: &str = "Snake";
const UI_COLOR: &str = "#000000";
const BG_COLOR: &str = "#439155";

const DEFAULT_FPS: u32 = 20;
const DEFAULT_LOG: &str = "snake.log";

const LOG_LEVEL: Level = Level::DEBUG;

#[derive(Parser)]
#[command(name = "Snake")]
struct Cli {
    #[arg(long, default_value_t=DEFAULT_FPS)]
    fps: u32,

    #[arg(long, default_value=DEFAULT_LOG)]
    log: String,
}

fn main() -> anyhow::Result<()> {
    let mut cli = Cli::parse();

    cli.log.insert_str(0, "logs/");
    tracing_subscriber::fmt()
        .with_writer(File::create(cli.log).context("creating log file")?)
        .with_max_level(LOG_LEVEL)
        .pretty()
        .init();

    let config = game::Config::new(TITLE.into(), UI_COLOR, BG_COLOR, cli.fps)
        .context("parsing command line arguments")?;

    game::init(config).context("rendering snake game")?;

    Ok(())
}
