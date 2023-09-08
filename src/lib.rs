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
    ops::{AddAssign, RangeInclusive},
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use thiserror::Error;
use tracing::{debug, instrument};

pub mod entity;
use entity::{Entity, Input, Sprite, Update, Vector};

const FPS_BOUNDS: RangeInclusive<u32> = 1..=30;
const DEFAULT_FPS: u32 = 15;
const DEFAULT_TITLE: &str = "Game";
const DEFAULT_UI_COLOR: &str = "#000000";
const DEFAULT_BG_COLOR: &str = "#666666";

const X_SCALE: i32 = 2; // compensate for squished sprites
const Y_SCALE: i32 = 1;

/// Error returned from the game.
/// Use UpdateError when `update` is called on an `Entity`.
#[derive(Error, Debug)]
pub enum GameError {
    #[error("error while updating entity: {}", .0)]
    UpdateError(String),

    #[error("entity rendered out of bounds")]
    OutOfBounds,

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Bmp(#[from] bmp::BmpError),

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

impl AddAssign<Vector> for Position {
    fn add_assign(&mut self, rhs: Vector) {
        self.x += rhs.x * X_SCALE;
        self.y += rhs.y * Y_SCALE;
    }
}

#[derive(Debug)]
struct EntityState {
    pos: Option<Position>,
    sprite: Rc<Sprite>,
    entity: Option<Box<dyn Entity>>,
}

impl EntityState {
    fn overlaps(&self, other: &Self) -> bool {
        let Position {
            x: self_x,
            y: self_y,
        } = self.pos.expect("self has a position");
        let Position {
            x: other_x,
            y: other_y,
        } = other.pos.expect("other has a position");

        self_x <= other_x + ((other.sprite.width() as i32 - 2) * X_SCALE)
            && self_x + ((self.sprite.width() as i32 - 2) * X_SCALE) >= other_x
            && self_y <= other_y + ((other.sprite.height() as i32 - 2) * Y_SCALE)
            && self_y + ((self.sprite.height() as i32 - 2) * Y_SCALE) >= other_y
    }

    fn within_bounds(&self, bounds: Rect) -> bool {
        let Position {
            x: self_x,
            y: self_y,
        } = self.pos.expect("self has a position");

        self_x > bounds.left() as i32 + 1
            && self_x + ((self.sprite.width() as i32 - 2) * X_SCALE) < bounds.right() as i32 - 1
            && self_y >= bounds.top() as i32
            && self_y + ((self.sprite.height() as i32 - 2) * Y_SCALE) < bounds.bottom() as i32
    }
}

#[derive(Debug)]
struct State {
    bounds: Option<Rect>,
    next_id: u32,
    entity_states: RefCell<HashMap<u32, RefCell<EntityState>>>,
}

impl State {
    fn new() -> Self {
        Self {
            bounds: None,
            next_id: 0,
            entity_states: RefCell::new(HashMap::new()),
        }
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = Some(bounds);
    }

    fn add_entity(&mut self, entity: Box<dyn Entity>) {
        let sprite = entity.sprite().clone();

        let entity_state = EntityState {
            pos: None,
            sprite,
            entity: Some(entity),
        };

        self.entity_states
            .borrow_mut()
            .insert(self.next_id, RefCell::new(entity_state));
        self.next_id += 1;
    }

    fn set_starting_positions(&mut self) -> Result<(), GameError> {
        let bounds = &self.bounds.expect("bounds should exist");

        for entity_state in self.entity_states.borrow_mut().values() {
            let mut entity_state = entity_state.borrow_mut();
            let (x, y) = entity_state
                .entity
                .as_ref()
                .expect("all entities should be Some")
                .start_pos();
            let sprite = &entity_state.sprite;

            // Position is the provided x/y positions times the screen width/height.
            // Subtract half the entity's width/height so position is middle of entity.
            let pos = Position {
                x: (((bounds.right() - bounds.left()) as f32 * x)
                    - ((sprite.width() * X_SCALE as u32) as f32 / 2.0)) as i32,
                y: (((bounds.bottom() - bounds.top()) as f32 * y)
                    - ((sprite.height() * Y_SCALE as u32) as f32 / 2.0)) as i32,
            };

            if !entity_state.within_bounds(self.bounds.unwrap()) {
                return Err(GameError::OutOfBounds);
            }

            entity_state.pos = Some(pos);
        }

        Ok(())
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

/// Game engine configuration builder.
#[derive(Debug)]
pub struct Engine {
    state: State,
    title: &'static str,
    ui_color: &'static str,
    bg_color: &'static str,
    fps: u32,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            state: State::new(),
            title: DEFAULT_TITLE,
            ui_color: DEFAULT_UI_COLOR,
            bg_color: DEFAULT_BG_COLOR,
            fps: DEFAULT_FPS,
        }
    }

    pub fn set_title(self, title: &'static str) -> Self {
        Self {
            title: title,
            ..self
        }
    }

    pub fn set_ui_color(self, ui_color: &'static str) -> Self {
        Self { ui_color, ..self }
    }
    pub fn set_bg_color(self, bg_color: &'static str) -> Self {
        Self { bg_color, ..self }
    }
    pub fn set_fps(self, fps: u32) -> Self {
        Self { fps, ..self }
    }

    pub fn starting_entities<T>(mut self, entities: T) -> Self
    where
        T: IntoIterator<Item = Box<dyn Entity>>,
    {
        for entity in entities {
            self.state.add_entity(entity);
        }
        self
    }

    /// Begin rendering the game using the provided `config` settings.
    #[instrument]
    pub fn init(mut self) -> Result<(), GameError> {
        let mut handle = TerminalHandle::new()?;
        let terminal = &mut handle.0;

        if !FPS_BOUNDS.contains(&self.fps) {
            return Err(GameError::InvalidArg(format!(
                "fps must be between {} and {}",
                FPS_BOUNDS.start(),
                FPS_BOUNDS.end()
            )));
        }
        let sleep_duration = Duration::from_secs_f32(1.0 / self.fps as f32);

        let ui_color = self.ui_color.parse()?;
        let bg_color = self.bg_color.parse()?;

        self.state.set_bounds(terminal.size()?);
        self.state.set_starting_positions()?;

        let game_border = Block::default()
            .title(format!(" {} ", self.title))
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
            .x_bounds([0.0, self.state.bounds.unwrap().width as f64])
            .y_bounds([0.0, self.state.bounds.unwrap().height as f64]);

        let input = Arc::new(Mutex::new(Input::None));

        // separate thread reads keyboard and updates the current input
        debug!("creating input reading thread");
        {
            let input = input.clone();

            thread::spawn(move || loop {
                *input.lock().unwrap() = read_input();
            });
        }

        let maybe_error = RefCell::new(None);
        loop {
            terminal.draw(|frame| {
                let canvas = canvas.clone().paint(|ctx| {
                    // render the entities, and hold onto any errors for outside the closures
                    if let Err(error) = render_entities(ctx, &self.state) {
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
            update_entities(*input, &self.state)?;
            *input = Input::None;

            // some entities may have been destroyed
            self.state
                .entity_states
                .borrow_mut()
                .retain(|_, entity_state| entity_state.borrow().entity.is_some());
        }
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

    for entity_state in state.entity_states.borrow().values() {
        let entity_state = entity_state.borrow();
        let pos = entity_state.pos.expect("entity has a position");
        let sprite = &entity_state.sprite;

        for x in 0..sprite.width() {
            for y in 0..sprite.height() {
                let rgb = sprite.get_pixel(x, y);
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
    }

    Ok(())
}

fn update_entities(input: Input, state: &State) -> Result<(), GameError> {
    for (&key, entity_state) in &*state.entity_states.borrow() {
        let mut entity_state = entity_state.borrow_mut();

        // Get mut borrows for all other entity states. The run-time borrow checking
        // will pass because even though entity_state has been mut borrowed already,
        // its key is used to filter it out from the iter.
        for mut other_entity_state in state
            .entity_states
            .borrow()
            .iter()
            .filter(|(&other_key, _)| other_key != key)
            .map(|(_, entity_state)| entity_state.borrow_mut())
        {
            if entity_state.overlaps(&other_entity_state) {
                if let Some(entity) = entity_state.entity.as_mut() {
                    if let Some(other_entity) = other_entity_state.entity.as_mut() {
                        entity.collision(other_entity);
                    }
                }
            }
        }

        let update = if let Some(entity) = entity_state.entity.as_mut() {
            entity.update(input)
        } else {
            Update::None
        };
        match update {
            Update::Move(vector) => {
                let old_pos = entity_state.pos;
                *entity_state.pos.as_mut().expect("entity has a position") += vector;

                if !entity_state.within_bounds(state.bounds.expect("bounds should exist")) {
                    entity_state.pos = old_pos;
                }
            }

            Update::Destroy => {
                entity_state.entity = None;
            }

            Update::None => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {}
