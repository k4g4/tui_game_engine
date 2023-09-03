use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    prelude::*,
    style::ParseColorError,
    text::Line as TextLine,
    widgets::{
        canvas::{Canvas, Context, Line},
        Block, BorderType, Borders,
    },
    Terminal,
};
use std::{cell::RefCell, io, ops::RangeInclusive, rc::Rc, time::Duration};
use thiserror::Error;
use tokio::{join, time};

const FPS_BOUNDS: RangeInclusive<u32> = 1..=10;

/// Configuration settings for the game.
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
    }
}

trait Entity {}

struct State {
    width: f64,
    height: f64,
    ui_color: Color,
    entities: Vec<Box<dyn Entity>>,
}

impl State {
    fn new(width: f64, height: f64, ui_color: Color) -> Self {
        Self {
            width,
            height,
            ui_color,
            entities: vec![],
        }
    }
}

enum Input {
    None,
    Up,
    Down,
    Left,
    Right,
    Quit,
}

/// Begin rendering the game using the provided `config` settings.
pub async fn init(config: Config) -> Result<(), GameError> {
    let mut handle = TerminalHandle::new()?;
    let terminal = &mut handle.0;

    let mut stream = EventStream::new();

    let sleep_duration = Duration::from_secs_f32(1_f32 / config.fps as f32);

    let ui_color = config.ui_color.parse()?;
    let bg_color = config.bg_color.parse()?;

    let Rect { width, height, .. } = terminal.size()?;
    let state = Rc::new(RefCell::new(State::new(
        width as f64,
        height as f64,
        ui_color,
    )));

    let game_border = Block::default()
        .title(format!(" {} ", config.title))
        .title_style(Style::default().add_modifier(Modifier::BOLD).fg(ui_color))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(ui_color))
        .style(Style::default().bg(bg_color));

    let canvas_template = Canvas::default()
        .block(game_border)
        .background_color(bg_color)
        .marker(Marker::Block)
        .x_bounds([0f64, width as f64])
        .y_bounds([0f64, height as f64]);

    loop {
        terminal.draw(|frame| {
            let canvas = canvas_template.clone().paint(|ctx| {
                render_frame(ctx, state.clone());
            });
            frame.render_widget(canvas, frame.size());
        })?;

        // future returns every single sleep_duration, and a key press might be returned
        if let (Ok(Some(result)), _) = join!(
            time::timeout(sleep_duration, stream.next()),
            time::sleep(sleep_duration)
        ) {
            match handle_input(result?) {
                Input::Quit => break,
                _ => {}
            }
            struct Thing;
            impl Entity for Thing {}
            state.borrow_mut().entities.push(Box::new(Thing));
        }
    }

    Ok(())
}

fn render_frame(ctx: &mut Context, state: Rc<RefCell<State>>) {
    let state = state.borrow();
    let text = format!("Entities: {}", state.entities.len());

    ctx.print(
        0.5 * state.width,
        0.5 * state.height,
        TextLine::styled(text, Style::default().fg(state.ui_color).bold()),
    );
    ctx.draw(&Line::new(
        0.2 * state.width,
        0.8 * state.height,
        0.5 * state.width,
        0.1 * state.height,
        state.ui_color,
    ));
}

fn handle_input(event: Event) -> Input {
    if let Event::Key(key) = event {
        // quit the game if ctrl+c or q pressed
        if key.code == KeyCode::Char('q')
            || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
        {
            return Input::Quit;
        }

        let input = match key.code {
            KeyCode::Up => Input::Up,
            KeyCode::Down => Input::Down,
            KeyCode::Left => Input::Left,
            KeyCode::Right => Input::Right,
            _ => Input::None,
        };

        return input;
    }

    Input::None
}

#[cfg(test)]
mod tests {}
