//! Applying a chosen [`crate::actions::Action`], one module per group.
//!
//! Each handler takes `&mut GameState` and returns [`super::Applied`] saying
//! what the dispatcher should do next. They were arms of a single 1,277-line
//! `submit_action`.

pub(crate) mod abilities;
pub(crate) mod cast;
pub(crate) mod choices;
pub(crate) mod combat;
pub(crate) mod mulligan;
pub(crate) mod simple;
