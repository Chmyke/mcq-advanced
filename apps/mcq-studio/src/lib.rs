//! ROLE:    the teacher's Studio: write a paper visually, see it as a student will,
//!          check it live, add media, and publish it with its codes and room sheet.
//! OWNER:   `app::App` owns the screen state; `state::Studio` owns the project being edited.
//! INVARIANTS: every change goes through `state::Studio`; the paper is never published with
//!             errors; nothing typed or imported reaches a script; the draft stays in the browser.

pub mod actions;
pub mod app;
pub mod class_stats;
pub mod correct;
pub mod correction;
pub mod demo;
pub mod editor;
pub mod identity;
pub mod platform;
pub mod preview;
pub mod publish;
pub mod settings;
pub mod state;
pub mod texts;

pub use app::App;

/// The player's look, shared so the preview is exactly what students see.
pub const STYLE: &str = mcq_player::STYLE;
/// The Studio's own layout, on top of the player's look.
pub const STUDIO_STYLE: &str = include_str!("../web/studio.css");
