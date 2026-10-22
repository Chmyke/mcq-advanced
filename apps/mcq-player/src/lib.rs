//! ROLE:    the student player: opens a signed package, runs the exam, seals the copy.
//! OWNER:   `app::App` owns the screen state; the engine owns the session; nothing else holds state.
//! INVARIANTS: the student copy never contains answers before hand-in; the session is saved
//!             after every change; nothing typed or read from a paper reaches a script.

pub mod app;
pub mod b64;
pub mod link;
pub mod markdown;
pub mod platform;
pub mod render;
pub mod review;
pub mod texts;

pub use app::App;

/// The whole look, light and dark, in one stylesheet embedded in the program.
pub const STYLE: &str = include_str!("../web/style.css");
