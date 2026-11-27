#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Declared first: the `trf!` macro is used by all following modules.
#[macro_use]
mod i18n;
mod ai;
mod app;
mod background;
#[cfg(feature = "test-tools")]
mod diagnostics;
mod editor;
mod effects;
mod fonts;
mod inline_image;
mod model;
mod platform;
mod profiles;
mod remote;
mod ui;

use eframe::egui::{self, Color32, FontId, Key, Modifiers, Rect, Sense, Stroke};
use egui_term::{
    BackendSettings, FontSettings, PtyEvent, TerminalBackend, TerminalFont, TerminalView,
};
use model::{Axis, Node, Pane, Shell, Tab, Workspace};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

// Every module imports the crate root with `use crate::*;`, so the shared types
// below are reachable everywhere without long paths.
use app::*;
use i18n::tr;
use platform::*;
use ui::*;

/// Run a PTY test mode if one was requested. Returns the process exit code.
#[cfg(feature = "test-tools")]
fn run_diagnostics() -> Option<i32> {
    diagnostics::run_if_requested()
}

/// Release builds contain no test modes; refuse them instead of opening a window.
#[cfg(not(feature = "test-tools"))]
fn run_diagnostics() -> Option<i32> {
    if std::env::args()
        .skip(1)
        .any(|arg| arg.starts_with("--") && arg.ends_with("-test"))
    {
        eprintln!("Testmodi sind in diesem Build nicht enthalten.");
        return Some(2);
    }
    None
}

fn main() -> eframe::Result {
    if let Some(code) = run_diagnostics() {
        std::process::exit(code);
    }
    eframe::run_native(
        concat!("TerminalVelocity · ", env!("CARGO_PKG_VERSION")),
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1200.0, 760.0])
                .with_min_inner_size([800.0, 480.0])
                .with_transparent(true),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(TerminalApp::new(cc)))),
    )
}
