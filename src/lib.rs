use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{Block, BorderType, Borders},
    Terminal,
};
use std::{io, ops::RangeInclusive, time::Duration};
use thiserror::Error;

const FPS_BOUNDS: RangeInclusive<u32> = 1..=5;

pub struct Args {
    fps: u32,
}

#[derive(Error, Debug)]
pub enum GameError {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("bad argument: {}", .0)]
    BadArg(String),

    #[error("unknown error")]
    Unknown,
}

impl Args {
    pub fn new(fps: u32) -> Result<Self, GameError> {
        if !FPS_BOUNDS.contains(&fps) {
            return Err(GameError::BadArg(format!(
                "fps must be between {} and {}",
                FPS_BOUNDS.start(), FPS_BOUNDS.end()
            )));
        }

        Ok(Self { fps })
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
                return Err(GameError::Io(error));
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

pub async fn render(args: Args) -> Result<(), GameError> {
    let mut handle = RenderHandle::new()?;
    let terminal = &mut handle.terminal;
    let sleep_duration = Duration::from_secs_f32(1 as f32 / args.fps as f32);

    for i in 0..20 {
        terminal.draw(|frame| {
            frame.render_widget(
                Block::default()
                    .title(i.to_string())
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick),
                frame.size(),
            );
        })?;

        tokio::time::sleep(sleep_duration).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {}
