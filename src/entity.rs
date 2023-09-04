use std::fmt::{Debug, Formatter, Result};

/// Input received from the player.
#[derive(Copy, Clone, Debug)]
pub enum Input {
    None,
    Up,
    Down,
    Left,
    Right,
    Quit,
}

/// Used for entities to specify movements/directions.
#[derive(Copy, Clone, Default, Debug)]
pub struct Vector {
    pub x: u32,
    pub y: u32,
}

impl Vector {
    pub fn new(x: u32, y: u32) -> Self {
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
    height: u32,
    width: u32,
    pixels: Box<[(u8, u8, u8)]>,
}

impl Sprite {
    pub fn new(height: u32, width: u32, bytes: &[u8]) -> Self {
        let pixels = bytes
            .chunks(3)
            .map(|chunk| (chunk[0], chunk[1], chunk[2]))
            .collect();

        Self {
            height,
            width,
            pixels,
        }
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn get_pixel_color(&self, x: u32, y: u32) -> (u8, u8, u8) {
        self.pixels[(y * self.height + x) as usize]
    }
}

impl Debug for Sprite {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("Sprite")
            .field("height", &self.height)
            .field("width", &self.width)
            .finish()
    }
}

/// A game entity of some kind.
pub trait Entity: Debug {
    /// Update the entity for this game tick.
    fn update(&mut self, input: Input) -> Update;
    /// Get the entity's sprite.
    fn sprite(&self) -> &Sprite;
}
