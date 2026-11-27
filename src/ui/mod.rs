//! Dialogs, bars and pane drawing. Each module adds `impl TerminalApp` methods.

mod help;
mod palette;
mod panes;
mod profile_dialog;
mod remote_dialog;
mod search;

pub(crate) use palette::*;
pub(crate) use panes::*;
pub(crate) use search::*;
