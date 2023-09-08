use bmp::Image;
use std::{
    fmt::{self, Debug, Formatter},
    path::Path,
    rc::Rc,
};

use crate::GameError;

/// Input received from the player.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Input {
    None,
    Up,
    Down,
    Left,
    Right,
    Quit,
}

/// Used for entities to specify movements/directions.
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct Vector {
    pub x: i32,
    pub y: i32,
}

impl Vector {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Copy, Clone, Default, Debug)]
pub enum Update {
    #[default]
    None,
    Move(Vector),
    Destroy,
}

#[derive(Copy, Clone, Debug)]
pub enum Effect {
    Damage(i32),
}

/// A game entity's sprite used for rendering.
pub struct Sprite {
    image: Image,
}

impl Sprite {
    pub fn new(path: &Path) -> Result<Self, GameError> {
        Ok(Self {
            image: bmp::open(path)?,
        })
    }

    pub fn width(&self) -> u32 {
        self.image.get_width()
    }

    pub fn height(&self) -> u32 {
        self.image.get_height()
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let pixel = self.image.get_pixel(x, y);
        (pixel.r, pixel.g, pixel.b)
    }
}

impl Debug for Sprite {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sprite")
            .field("width", &self.width())
            .field("height", &self.height())
            .finish()
    }
}

/// A game entity of some kind.
pub trait Entity: Debug {
    /// Starting position for the entity, between [0, 1).
    fn start_pos(&self) -> (f32, f32);

    /// Get the entity's sprite.
    fn sprite(&self) -> &Rc<Sprite>;

    /// Update the entity for this game tick.
    fn update(&mut self, input: Input) -> Update;

    /// Respond to a collision with another entity.
    fn collision(&mut self, other: &mut Box<dyn Entity>);

    /// Respond to an effect.
    fn effect(&mut self, effect: Effect);
}
