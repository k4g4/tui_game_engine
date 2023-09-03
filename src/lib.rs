use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    prelude::*,
    widgets::{
        canvas::{Canvas, Line},
        Block, BorderType, Borders,
    },
    Terminal,
};
use std::{io, ops::RangeInclusive, time::Duration};
use thiserror::Error;
use tokio::{join, time};

const FPS_BOUNDS: RangeInclusive<u32> = 1..=10;

/// Configuration settings for the game.
pub struct Config {
    title: String,
    ui_color: (u8, u8, u8),
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
    pub fn new(
        title: String,
        ui_color: (u8, u8, u8),
        bg_color: (u8, u8, u8),
        fps: u32,
    ) -> Result<Self, GameError> {
        if !FPS_BOUNDS.contains(&fps) {
            return Err(GameError::BadArg(format!(
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

    fn get_color(color: (u8, u8, u8)) -> Color {
        Color::Rgb(color.0, color.1, color.2)
    }
    let ui_color = get_color(config.ui_color);
    let bg_color = get_color(config.bg_color);

    let game_border = Block::default()
        .title(format!(" {} ", config.title))
        .title_style(Style::default().add_modifier(Modifier::BOLD).fg(ui_color))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(ui_color))
        .style(Style::default().bg(bg_color));

    // some math to determine coords for each x and y
    let Rect { width, height, .. } = terminal.size()?;

    let get_x = |x: f64| -> f64 { x * width as f64 };

    let get_y = |y: f64| -> f64 { y * height as f64 };

    let canvas_template = Canvas::default()
        .block(game_border)
        .background_color(bg_color)
        .marker(Marker::Block)
        .x_bounds([0f64, width as f64])
        .y_bounds([0f64, height as f64]);

    loop {
        terminal.draw(|frame| {
            let canvas = canvas_template.clone().paint(|ctx| {
                ctx.print(get_x(0.5), get_y(0.5), "foobar".fg(ui_color).bold());
                ctx.draw(&Line {
                    x1: get_x(0.2),
                    y1: get_y(0.8),
                    x2: get_x(0.5),
                    y2: get_y(0.1),
                    color: ui_color,
                });
            });
            frame.render_widget(canvas, frame.size());
        })?;

        // future returns every single sleep_duration, and a key press might be returned
        if let (Ok(Some(result)), _) = join!(
            time::timeout(sleep_duration, stream.next()),
            time::sleep(sleep_duration)
        ) {
            if let Event::Key(key) = result? {
                // quit the game if ctrl+c or q pressed
                if key.code == KeyCode::Char('q')
                    || (key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c'))
                {
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
