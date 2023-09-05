use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use tracing::Level;

use game::{
    entity::{Entity, Input, Sprite, Update, Vector},
    GameError,
};

const TITLE: &str = "Snake";
const UI_COLOR: &str = "#000000";
const BG_COLOR: &str = "#439155";

const LOG_DIR: &str = "logs";
const LOG_LEVEL: Level = Level::DEBUG;

const BMPS_DIR: &str = "bmps";
const SMILEY_BMP: &str = "smiley.bmp";

const DEFAULT_FPS: u32 = 10;
const DEFAULT_LOG: &str = "snake.log";

#[derive(Parser)]
#[command(name = "Snake")]
struct Cli {
    #[arg(long, default_value_t=DEFAULT_FPS)]
    fps: u32,

    #[arg(long, default_value=DEFAULT_LOG)]
    log: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // log to the specified log file, or default to LOG_DIR/DEFAULT_LOG
    let log_path =
        Path::new(LOG_DIR).join(cli.log.file_name().ok_or(anyhow!("invalid log filename"))?);
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

    #[derive(Debug)]
    struct Player(Sprite);

    impl Entity for Player {
        fn update(&mut self, input: Input) -> Result<Update, GameError> {
            Ok(match input {
                Input::Up => Update::Move(Vector::new(0, 2)),
                Input::Down => Update::Move(Vector::new(0, -2)),
                Input::Left => Update::Move(Vector::new(-2, 0)),
                Input::Right => Update::Move(Vector::new(2, 0)),
                _ => Update::None,
            })
        }

        fn sprite(&self) -> &Sprite {
            &self.0
        }
    }

    let smiley_path = [BMPS_DIR, SMILEY_BMP].iter().collect::<PathBuf>();
    let player = Player(get_sprite(&smiley_path)?);

    let config = game::Config::new(
        TITLE.into(),
        UI_COLOR,
        BG_COLOR,
        cli.fps,
        vec![Box::new(player)],
    )
    .context("while parsing command line arguments")?;

    if let Err(error) = game::init(config).context("while rendering snake game") {
        // since the terminal has been hijacked, print errors to the log
        tracing::debug!("Error: {:?}", error);
        return Err(error);
    }

    Ok(())
}

fn get_sprite(path: &Path) -> Result<Sprite> {
    let img = bmp::open(path)?;
    let (width, height) = (img.get_width(), img.get_height());

    let mut bytes =
        Vec::with_capacity(std::mem::size_of::<bmp::Pixel>() * height as usize * width as usize);
    bytes.extend(
        img.coordinates()
            .map(|(x, y)| img.get_pixel(x, height - y - 1))
            .flat_map(|pixel| [pixel.r, pixel.g, pixel.b]),
    );

    Ok(Sprite::new(width, height, &bytes))
}
