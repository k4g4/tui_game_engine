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
        canvas::{Canvas, Context, Painter},
        Block, BorderType, Borders,
    },
    Terminal,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    io,
    ops::{Add, RangeInclusive},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use thiserror::Error;
use tracing::{debug, instrument};

pub mod entity;
use entity::{Entity, Input, Update, Vector};

const FPS_BOUNDS: RangeInclusive<u32> = 1..=20;

const X_SCALE: i32 = 2; // compensate for squished sprites
const Y_SCALE: i32 = 1;

/// Configuration settings for the game.
#[derive(Debug)]
pub struct Config {
    title: String,
    ui_color: &'static str,
    bg_color: &'static str,
    fps: u32,
    entities: Vec<Box<dyn Entity>>,
}

impl Config {
    pub fn new(
        title: String,
        ui_color: &'static str,
        bg_color: &'static str,
        fps: u32,
        entities: Vec<Box<dyn Entity>>,
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
            entities,
        })
    }
}

/// Error returned from the game.
/// Use UpdateError when `update` is called on an `Entity`.
#[derive(Error, Debug)]
pub enum GameError {
    #[error("error while updating entity: {}", .0)]
    UpdateError(String),

    #[error("sprite rendered out of bounds")]
    OutOfBounds,

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    InvalidColor(#[from] ParseColorError),

    #[error("invalid argument: {}", .0)]
    InvalidArg(String),

    #[error("unknown error")]
    Unknown,
}

#[derive(Copy, Clone, Debug)]
struct Position {
    x: i32,
    y: i32,
}

impl Add<Vector> for Position {
    type Output = Self;

    fn add(self, rhs: Vector) -> Self::Output {
        Self::Output {
            x: self.x + (rhs.x * X_SCALE),
            y: self.y + (rhs.y * Y_SCALE),
        }
    }
}

struct EntityState {
    pos: Position,
    entity: Box<dyn Entity>,
}

struct State {
    bounds: Rect,
    next_id: u32,
    entity_states: HashMap<u32, RefCell<EntityState>>,
}

impl State {
    fn new(bounds: Rect) -> Self {
        Self {
            bounds,
            next_id: 0,
            entity_states: HashMap::new(),
        }
    }

    fn add_entity(&mut self, entity: Box<dyn Entity>, pos: Position) {
        self.entity_states
            .insert(self.next_id, RefCell::new(EntityState { pos, entity }));
        self.next_id += 1;
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

/// Begin rendering the game using the provided `config` settings.
#[instrument]
pub fn init(config: Config) -> Result<(), GameError> {
    let mut handle = TerminalHandle::new()?;
    let terminal = &mut handle.0;

    let sleep_duration = Duration::from_secs_f32(1.0 / config.fps as f32);

    let ui_color = config.ui_color.parse()?;
    let bg_color = config.bg_color.parse()?;

    let bounds = terminal.size()?;

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
        .x_bounds([0.0, bounds.width as f64])
        .y_bounds([0.0, bounds.height as f64]);

    let input = Arc::new(Mutex::new(Input::None));

    // separate thread reads keyboard and updates the current input
    {
        debug!("creating input reading thread");

        let input = input.clone();
        thread::spawn(move || loop {
            *input.lock().unwrap() = read_input();
        });
    }

    let mut state = State::new(bounds);
    for entity in config.entities {
        state.add_entity(entity, Position { x: 10, y: 10 });
    }

    let maybe_error = RefCell::new(None);
    loop {
        terminal.draw(|frame| {
            let canvas = canvas.clone().paint(|ctx| {
                // render the entities, and hold onto any errors for outside the closures
                if let Err(error) = render_entities(ctx, &state) {
                    *maybe_error.borrow_mut() = Some(error);
                }

                ctx.layer();
            });

            frame.render_widget(canvas, frame.size());
        })?;
        if let Some(error) = maybe_error.borrow_mut().take() {
            return Err(error);
        }

        thread::sleep(sleep_duration);

        let mut input = input.lock().expect("not poisoned");
        if *input == Input::Quit {
            return Ok(());
        }
        update_entities(*input, &state)?;
        *input = Input::None;
    }
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

fn render_entities(ctx: &mut Context, state: &State) -> Result<(), GameError> {
    let mut painter = Painter::from(ctx);

    for entity_state in state.entity_states.values() {
        let entity_state = entity_state.borrow();
        let pos = entity_state.pos;
        let sprite = entity_state.entity.sprite();

        for x in 0..sprite.width() {
            for y in 0..sprite.height() {
                let rgb = sprite.get_pixel_color(x, y);
                let color = Color::Rgb(rgb.0, rgb.1, rgb.2);

                let (x_offset, y_offset) = painter
                    .get_point(
                        (pos.x + (x as i32 * X_SCALE)) as f64,
                        (pos.y + (y as i32 * Y_SCALE)) as f64,
                    )
                    .ok_or(GameError::OutOfBounds)?;

                // sprites will look squished unless scaling factor is accounted for
                for x in 0..X_SCALE {
                    for y in 0..Y_SCALE {
                        painter.paint(x_offset - x as usize, y_offset - y as usize, color);
                    }
                }
            }
        }

        // uncomment to show pos of entity
        // if let Some((x, y)) = painter.get_point(pos.x as f64, pos.y as f64) {
        //     painter.paint(x, y, Color::Magenta);
        // }
    }

    Ok(())
}

fn update_entities(input: Input, state: &State) -> Result<(), GameError> {
    for entity_state in state.entity_states.values() {
        let mut entity_state = entity_state.borrow_mut();

        match entity_state.entity.update(input)? {
            Update::Move(vector) => {
                let new_pos = entity_state.pos + vector;
                let (width, height) = entity_state.entity.dimensions();

                if new_pos.x >= state.bounds.left() as i32 + 1
                    && new_pos.x + ((width as i32 - 2) * X_SCALE) < state.bounds.right() as i32
                    && new_pos.y >= state.bounds.top() as i32
                    && new_pos.y + ((height as i32 - 2) * Y_SCALE) < state.bounds.bottom() as i32
                {
                    entity_state.pos = new_pos;
                }
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {}
