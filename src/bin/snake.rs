use anyhow::{anyhow, Context};
use clap::Parser;
use std::{fs::File, path::PathBuf};
use tracing::Level;

use game::entity::{Entity, Sprite, Update, Input};

const TITLE: &str = "Snake";
const UI_COLOR: &str = "#000000";
const BG_COLOR: &str = "#439155";

const DEFAULT_FPS: u32 = 1;
const DEFAULT_LOG: &str = "snake.log";

const LOG_DIR: &str = "logs/";
const LOG_LEVEL: Level = Level::DEBUG;

const SMILEY_BMP: &str = "bmps/smiley.bmp";

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
        .or_else(|_| {
            std::fs::create_dir(LOG_DIR)?;
            File::create(&log_path).context("while creating log file")
        })?;
    tracing_subscriber::fmt()
        .with_writer(log)
        .with_max_level(LOG_LEVEL)
        .pretty()
        .init();

    let smiley = {
        let img = bmp::open(SMILEY_BMP)?;
        let (height, width) = (img.get_height(), img.get_width());
        let mut bytes = Vec::with_capacity(
            std::mem::size_of::<bmp::Pixel>() * height as usize * width as usize,
        );
        img.to_writer(&mut bytes)?;

        Sprite::new(height, width, &bytes)
    };

    #[derive(Debug)]
    struct Player(Sprite);

    impl Entity for Player {
        fn update(&mut self, _input: Input) -> Update {
            Update::default()
        }

        fn sprite(&self) -> &Sprite {
            &self.0
        }
    }

    let config = game::Config::new(
        TITLE.into(),
        UI_COLOR,
        BG_COLOR,
        cli.fps,
        vec![Box::new(Player(smiley))],
    )
    .context("while parsing command line arguments")?;

    game::init(config).context("while rendering snake game")?;

    Ok(())
}
