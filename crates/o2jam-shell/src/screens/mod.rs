//! Window screens: scene painters plus the [`ScreenGame`] loop.

pub mod game;
pub mod paint;

pub use game::ScreenGame;
pub use paint::paint_screen;
