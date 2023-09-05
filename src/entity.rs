use crate::GameError;
use std::fmt::{Debug, Formatter, Result as FmtResult};

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
}

/// A game entity's sprite used for rendering.
pub struct Sprite {
    width: u32,
    height: u32,
    pixels: Box<[(u8, u8, u8)]>,
}

impl Sprite {
    /// Height, width, and byte data for the sprite.
    /// `bytes.len()` must be modulo 3 to extract RGB values for each pixel.
    /// Order of each triplet: [y0_x0, y0_x1, .., y0_xn, y1_x0, y1_x1, .., ym_xn]
    pub fn new(width: u32, height: u32, bytes: &[u8]) -> Self {
        let pixels = bytes
            .chunks(3)
            .map(|chunk| (chunk[0], chunk[1], chunk[2]))
            .collect();

        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn get_pixel_color(&self, x: u32, y: u32) -> (u8, u8, u8) {
        self.pixels[(y * self.width + x) as usize]
    }
}

impl Debug for Sprite {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("Sprite")
            .field("height", &self.height)
            .field("width", &self.width)
            .finish()
    }
}

/// A game entity of some kind.
pub trait Entity: Debug {
    /// Starting position for the entity, between [0, 1).
    fn start_pos(&self) -> (f32, f32);

    /// Update the entity for this game tick.
    fn update(&mut self, input: Input) -> Result<Update, GameError>;

    /// Get the entity's sprite.
    fn sprite(&self) -> &Sprite;

    /// Get entity's hitbox dimensions.
    fn dimensions(&self) -> (u32, u32) {
        (self.sprite().width, self.sprite().height)
    }
}
