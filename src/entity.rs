use ratatui::{style::Color, widgets::canvas::Painter};

use crate::{Input, State};

/// A game entity of some kind.
pub trait Entity {
    /// Update the entity for this game tick.
    fn update(&mut self, input: Input, state: &State);
    /// Render to the screen using the provided Context.
    fn render(&self, painter: &mut Painter);
}
