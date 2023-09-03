use ratatui::{
    style::{Color, Style},
    text::Line as TextLine,
    widgets::canvas::{Context, Line as CanvasLine},
};

use crate::Input;

/// A game entity of some kind.
pub trait Entity {
    /// Handle player input.
    fn handle_input(&mut self, input: Input);
    /// Render to the screen using the provided Context.
    fn render(&self, ctx: &mut Context);
}

#[derive(Clone, Copy)]
pub struct Point {
    x: f64,
    y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

pub struct Line {
    ends: (Point, Point),
    color: Color,
}

impl Line {
    pub fn new(ends: (Point, Point), color: Color) -> Self {
        Self { ends, color }
    }
}

impl Entity for Line {
    fn handle_input(&mut self, input: Input) {
        match input {
            Input::Up => {
                self.ends.0.y += 5f64;
                self.ends.1.y += 5f64;
            }
            Input::Down => {
                self.ends.0.y -= 5f64;
                self.ends.1.y -= 5f64;
            }
            Input::Left => {
                self.ends.0.x -= 5f64;
                self.ends.1.x -= 5f64;
            }
            Input::Right => {
                self.ends.0.x += 5f64;
                self.ends.1.x += 5f64;
            }
            _ => {}
        }
    }

    fn render(&self, ctx: &mut Context) {
        ctx.draw(&CanvasLine::new(
            self.ends.0.x,
            self.ends.0.y,
            self.ends.1.x,
            self.ends.1.y,
            self.color,
        ));
    }
}

pub struct TextBox {
    text: String,
    pos: Point,
    style: Style,
}

impl TextBox {
    pub fn new(text: String, pos: Point, style: Style) -> Self {
        Self { text, pos, style }
    }
}

impl Entity for TextBox {
    fn handle_input(&mut self, _input: Input) {}
    fn render(&self, ctx: &mut Context) {
        ctx.print(
            self.pos.x,
            self.pos.y,
            TextLine::styled(self.text.clone(), self.style),
        );
    }
}
