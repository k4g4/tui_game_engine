use anyhow::{anyhow, Context};
use clap::Parser;
use std::{fs::File, path::PathBuf};
use tracing::Level;

const TITLE: &str = "Snake";
const UI_COLOR: &str = "#000000";
const BG_COLOR: &str = "#439155";

const DEFAULT_FPS: u32 = 20;
const DEFAULT_LOG: &str = "snake.log";

const LOG_DIR: &str = "vlogs/";
const LOG_LEVEL: Level = Level::DEBUG;

#[derive(Parser)]
#[command(name = "Snake")]
struct Cli {
    #[arg(long, default_value_t=DEFAULT_FPS)]
    fps: u32,

    #[arg(long, default_value=DEFAULT_LOG)]
    log: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let log_path =
        PathBuf::from(LOG_DIR).join(cli.log.file_name().ok_or(anyhow!("invalid log filename"))?);
    let log = File::create(&log_path)
        .context("while creating log file")
        .or_else(|err| {
            std::fs::create_dir(LOG_DIR)?;
            File::create(&log_path).context("while creating log file")
        })?;
    tracing_subscriber::fmt()
        .with_writer(log)
        .with_max_level(LOG_LEVEL)
        .pretty()
        .init();

    let config = game::Config::new(TITLE.into(), UI_COLOR, BG_COLOR, cli.fps)
        .context("while parsing command line arguments")?;

    game::init(config).context("while rendering snake game")?;

    Ok(())
}
