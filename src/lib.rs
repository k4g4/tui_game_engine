use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    prelude::*,
    widgets::{Block, BorderType, Borders},
    Terminal,
};
use std::{io, ops::RangeInclusive, time::Duration};
use thiserror::Error;
use tokio::{join, time};

const FPS_BOUNDS: RangeInclusive<u32> = 1..=10;

/// Configuration settings for the game.
pub struct Config {
    title: String,
    bg_color: (u8, u8, u8),
    fps: u32,
}

/// Error returned from the game.
#[derive(Error, Debug)]
pub enum GameError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("bad argument: {}", .0)]
    BadArg(String),

    #[error("unknown error")]
    Unknown,
}

impl Config {
    pub fn new(title: String, bg_color: (u8, u8, u8), fps: u32) -> Result<Self, GameError> {
        if !FPS_BOUNDS.contains(&fps) {
            return Err(GameError::BadArg(format!(
                "fps must be between {} and {}",
                FPS_BOUNDS.start(),
                FPS_BOUNDS.end()
            )));
        }

        Ok(Self { title, bg_color, fps })
    }
}

struct RenderHandle {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl RenderHandle {
    fn new() -> Result<Self, GameError> {
        // need to make sure disable_raw_mode is always called if any error occurs

        enable_raw_mode()?;

        let mut stdout = io::stdout();

        if let Err(error) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
            disable_raw_mode()?;
            return Err(GameError::Io(error));
        }

        let backend = CrosstermBackend::new(stdout);

        let terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(error) => {
                disable_raw_mode()?;
                return Err(error.into());
            }
        };

        Ok(Self { terminal })
    }
}

impl Drop for RenderHandle {
    fn drop(&mut self) {
        // RAII guard to ensure terminal settings reset

        disable_raw_mode().expect("raw mode enabled, so it should disable");

        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .expect("leaving alt screen and disabling mouse capture");

        self.terminal.show_cursor().expect("showing cursor");
    }
}

enum Input {
    None,
    Up,
    Down,
    Left,
    Right,
}

/// Begin rendering the game using the provided `config` settings.
pub async fn init(config: Config) -> Result<(), GameError> {
    let mut handle = RenderHandle::new()?;
    let terminal = &mut handle.terminal;

    let mut stream = crossterm::event::EventStream::new();
    let sleep_duration = Duration::from_secs_f32(1_f32 / config.fps as f32);

    let color = Color::Rgb(config.bg_color.0, config.bg_color.1, config.bg_color.2);
    let game_widget = Block::default()
        .title(format!(" {} ", config.title))
        .title_style(Style::default().add_modifier(Modifier::BOLD))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .style(Style::default().bg(color));

    loop {
        terminal.draw(|frame| {
            frame.render_widget(game_widget.clone(), frame.size());
        })?;

        // future returns every single sleep_duration, and a key press might be returned
        if let (Ok(Some(result)), _) = join!(
            time::timeout(sleep_duration, stream.next()),
            time::sleep(sleep_duration)
        ) {
            if let Event::Key(key) = result? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    return Ok(());
                }

                let _input = match key.code {
                    KeyCode::Up => Input::Up,
                    KeyCode::Down => Input::Down,
                    KeyCode::Left => Input::Left,
                    KeyCode::Right => Input::Right,
                    _ => Input::None,
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {}
