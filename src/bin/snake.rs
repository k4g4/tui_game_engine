use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::{
    fs::File,
    path::{Path, PathBuf},
    rc::Rc,
};
use tracing::Level;

use game::{
    entity::{Effect, Entity, Input, Sprite, Update, Vector},
    Engine,
};

const TITLE: &str = "Snake";
const UI_COLOR: &str = "#000000";
const BG_COLOR: &str = "#439155";

const LOG_DIR: &str = "logs";
const LOG_LEVEL: Level = Level::DEBUG;

const BMPS_DIR: &str = "bmps";
const SMILEY_BMP: &str = "smiley.bmp";
const MEANIE_BMP: &str = "meanie.bmp";

const DEFAULT_FPS: u32 = 15;
const DEFAULT_LOG: &str = "snake.log";

#[derive(Parser)]
#[command(name = "Snake")]
struct Cli {
    #[arg(long, default_value_t=DEFAULT_FPS)]
    fps: u32,

    #[arg(long, default_value=DEFAULT_LOG)]
    log: PathBuf,
}

#[derive(Debug)]
struct Player((f32, f32), Rc<Sprite>, i32);

impl Entity for Player {
    fn start_pos(&self) -> (f32, f32) {
        self.0
    }

    fn update(&mut self, input: Input) -> Update {
        if self.2 <= 0 {
            return Update::Destroy;
        }

        match input {
            Input::Up => Update::Move(Vector::new(0, 2)),
            Input::Down => Update::Move(Vector::new(0, -2)),
            Input::Left => Update::Move(Vector::new(-2, 0)),
            Input::Right => Update::Move(Vector::new(2, 0)),
            _ => Update::None,
        }
    }

    fn sprite(&self) -> &Rc<Sprite> {
        &self.1
    }

    fn collision(&mut self, other: &mut Box<dyn Entity>) {
        other.effect(Effect::Damage(10));
    }

    fn effect(&mut self, effect: Effect) {
        match effect {
            _ => {}
        }
    }
}

#[derive(Debug)]
struct Enemy((f32, f32), Rc<Sprite>, i32);

impl Entity for Enemy {
    fn start_pos(&self) -> (f32, f32) {
        self.0
    }

    fn update(&mut self, input: Input) -> Update {
        if self.2 <= 0 {
            return Update::Destroy;
        }

        match input {
            _ => Update::None,
        }
    }

    fn sprite(&self) -> &Rc<Sprite> {
        &self.1
    }

    fn collision(&mut self, _other: &mut Box<dyn Entity>) {}

    fn effect(&mut self, effect: Effect) {
        match effect {
            Effect::Damage(damage) => {
                self.2 -= damage;
            }
        }
    }
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

    let smiley_path = Path::new(BMPS_DIR).join(SMILEY_BMP);
    let meanie_path = Path::new(BMPS_DIR).join(MEANIE_BMP);
    let smiley = Rc::new(Sprite::new(&smiley_path)?);
    let meanie = Rc::new(Sprite::new(&meanie_path)?);

    let mut entities: Vec<_> = [
        (0.2, 0.2),
        (0.5, 0.2),
        (0.8, 0.2),
        (0.2, 0.8),
        (0.5, 0.8),
        (0.8, 0.8),
    ]
    .into_iter()
    .map(|pos| Box::new(Enemy(pos, meanie.clone(), 5)) as Box<dyn Entity>)
    .collect();

    entities.push(Box::new(Player((0.5, 0.5), smiley.clone(), 10)));

    if let Err(error) = Engine::new()
        .set_title(TITLE)
        .set_ui_color(UI_COLOR)
        .set_bg_color(BG_COLOR)
        .starting_entities(entities)
        .init()
        .context("while rendering snake game")
    {
        // since the terminal has been hijacked, print errors to the log
        tracing::debug!("Error: {:?}", error);
        return Err(error);
    }

    Ok(())
}
