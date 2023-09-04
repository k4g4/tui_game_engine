use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    prelude::*,
    style::ParseColorError,
    widgets::{
        canvas::{Canvas, Painter},
        Block, BorderType, Borders,
    },
    Terminal,
};
use std::{
    cell::RefCell,
    io,
    ops::RangeInclusive,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use thiserror::Error;
use tracing::{instrument, debug};

mod entity;
use entity::Entity;

const FPS_BOUNDS: RangeInclusive<u32> = 1..=20;

/// Configuration settings for the game.
#[derive(Debug)]
pub struct Config {
    title: String,
    ui_color: &'static str,
    bg_color: &'static str,
    fps: u32,
}

/// Error returned from the game.
#[derive(Error, Debug)]
pub enum GameError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    InvalidColor(#[from] ParseColorError),

    #[error("invalid argument: {}", .0)]
    InvalidArg(String),

    #[error("unknown error")]
    Unknown,
}

impl Config {
    pub fn new(
        title: String,
        ui_color: &'static str,
        bg_color: &'static str,
        fps: u32,
    ) -> Result<Self, GameError> {
        if !FPS_BOUNDS.contains(&fps) {
            return Err(GameError::InvalidArg(format!(
                "fps must be between {} and {}",
                FPS_BOUNDS.start(),
                FPS_BOUNDS.end()
            )));
        }

        Ok(Self {
            title,
            ui_color,
            bg_color,
            fps,
        })
    }
}

struct TerminalHandle(Terminal<CrosstermBackend<io::Stdout>>);

impl TerminalHandle {
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

        debug!("terminal handle constructed");

        Ok(Self(terminal))
    }
}

impl Drop for TerminalHandle {
    fn drop(&mut self) {
        // RAII guard to ensure terminal settings reset

        disable_raw_mode().expect("raw mode enabled, so it should disable");

        execute!(
            self.0.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .expect("leaving alt screen and disabling mouse capture");

        self.0.show_cursor().expect("showing cursor");

        debug!("terminal handle dropped");
    }
}

/// Input received from the player.
#[derive(Clone, Copy)]
pub enum Input {
    None,
    Up,
    Down,
    Left,
    Right,
    Quit,
}

pub struct State {
    entities: RefCell<Vec<Box<dyn Entity>>>,
}

impl State {
    fn new() -> Self {
        Self {
            entities: RefCell::new(vec![]),
        }
    }
}

/// Begin rendering the game using the provided `config` settings.
#[instrument]
pub fn init(config: Config) -> Result<(), GameError> {
    let mut handle = TerminalHandle::new()?;
    let terminal = &mut handle.0;

    let sleep_duration = Duration::from_secs_f32(1_f32 / config.fps as f32);

    let ui_color = config.ui_color.parse()?;
    let bg_color = config.bg_color.parse()?;

    let Rect { width, height, .. } = terminal.size()?;

    let game_border = Block::default()
        .title(format!(" {} ", config.title))
        .title_style(Style::default().add_modifier(Modifier::BOLD).fg(ui_color))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(ui_color))
        .style(Style::default().bg(bg_color));

    let canvas = Canvas::default()
        .block(game_border)
        .background_color(bg_color)
        .marker(Marker::Block)
        .x_bounds([0.0, width as f64])
        .y_bounds([0.0, height as f64]);

    let input = Arc::new(Mutex::new(Input::None));

    // separate thread reads keyboard and updates the current input
    {
        debug!("creating input reading thread");

        let input = input.clone();
        thread::spawn(move || loop {
            *input.lock().unwrap() = read_input();
        });
    }

    let state = State::new();
    {
        let _entities = &mut state.entities.borrow_mut();
    }

    loop {
        terminal.draw(|frame| {
            let canvas = canvas.clone().paint(|ctx| {
                let mut painter = Painter::from(ctx);
                for entity in state.entities.borrow().as_slice() {
                    entity.render(&mut painter);
                }
            });
            frame.render_widget(canvas, frame.size());
        })?;

        thread::sleep(sleep_duration);

        let mut input = input.lock().expect("not poisoned");
        match *input {
            Input::Quit => break,
            input => {
                for entity in &mut *state.entities.borrow_mut() {
                    entity.update(input, &state);
                }
            }
        }
        *input = Input::None;
    }

    debug!("game loop terminated");

    Ok(())
}

fn read_input() -> Input {
    let Event::Key(key) = crossterm::event::read().expect("reading event") else {
        return Input::None;
    };

    // quit the game if ctrl+c or q pressed
    if key.code == KeyCode::Char('q')
        || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
    {
        return Input::Quit;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('w') => Input::Up,
        KeyCode::Down | KeyCode::Char('s') => Input::Down,
        KeyCode::Left | KeyCode::Char('a') => Input::Left,
        KeyCode::Right | KeyCode::Char('d') => Input::Right,
        _ => Input::None,
    }
}

#[cfg(test)]
mod tests {}
