//! Command palette (Ctrl+Shift+P).

use crate::*;

#[derive(Default)]
pub(crate) struct CommandPalette {
    pub(crate) query: String,
    pub(crate) selected: usize,
    pub(crate) focus_input: bool,
}

#[derive(Clone)]
pub(crate) enum PaletteAction {
    NewTab(Shell),
    Split(Axis, Shell),
    OpenRemote(remote::Connection, usize),
    OpenProfile(usize, PathBuf),
    SwitchTab(usize),
    ActivateEditor,
    ShowSearch,
    ShowRemoteDialog,
    ShowProfiles(bool),
    SaveWorkspace,
    ToggleSettings,
    ToggleHelp,
    ToggleAbout,
    SetPreset(effects::Preset),
}

pub(crate) fn palette_matches_query(label: &str, query: &str) -> bool {
    let lower = label.to_lowercase();
    query
        .split_whitespace()
        .all(|word| lower.contains(&word.to_lowercase()))
}

impl TerminalApp {
    pub(crate) fn show_palette(&mut self) {
        self.close_search();
        self.palette = Some(CommandPalette {
            query: String::new(),
            selected: 0,
            focus_input: true,
        });
    }

    pub(crate) fn palette_entries(&self) -> Vec<(String, PaletteAction)> {
        let mut entries = Vec::new();
        for shell in Shell::ALL {
            entries.push((
                trf!("Neuer Tab: {shell}", shell = shell.label()),
                PaletteAction::NewTab(shell),
            ));
            if !self.editor.active {
                entries.push((
                    trf!("Nebeneinander teilen: {shell}", shell = shell.label()),
                    PaletteAction::Split(Axis::Columns, shell),
                ));
                entries.push((
                    trf!("Untereinander teilen: {shell}", shell = shell.label()),
                    PaletteAction::Split(Axis::Rows, shell),
                ));
            }
        }
        let mut conn_indices: Vec<_> = (0..self.workspace.connections.len()).collect();
        conn_indices.sort_by_key(|&i| !self.workspace.connections[i].favorite);
        for index in conn_indices {
            let c = &self.workspace.connections[index];
            let star = if c.favorite { "★ " } else { "" };
            entries.push((
                trf!(
                    "Verbinden (Neuer Tab): {star}{name} · {target}",
                    star = star,
                    name = c.name,
                    target = c.label()
                ),
                PaletteAction::OpenRemote(c.clone(), 0),
            ));
            if !self.editor.active {
                entries.push((
                    trf!(
                        "Verbinden (Nebeneinander): {star}{name} · {target}",
                        star = star,
                        name = c.name,
                        target = c.label()
                    ),
                    PaletteAction::OpenRemote(c.clone(), 1),
                ));
                entries.push((
                    trf!(
                        "Verbinden (Untereinander): {star}{name} · {target}",
                        star = star,
                        name = c.name,
                        target = c.label()
                    ),
                    PaletteAction::OpenRemote(c.clone(), 2),
                ));
            }
        }
        for (index, profile) in self.profiles.profiles.iter().enumerate() {
            entries.push((
                trf!(
                    "Projekt öffnen: {name} ({directory})",
                    name = profile.name,
                    directory = profile.directory.display()
                ),
                PaletteAction::OpenProfile(index, profile.directory.clone()),
            ));
        }
        for (index, tab) in self.workspace.tabs.iter().enumerate() {
            entries.push((
                trf!(
                    "Wechseln zu Tab {number}: {name}",
                    number = index + 1,
                    name = tab.name
                ),
                PaletteAction::SwitchTab(index),
            ));
        }
        entries.extend([
            (tr("Editor öffnen").into(), PaletteAction::ActivateEditor),
            (
                tr("Verlauf durchsuchen (Strg+F)").into(),
                PaletteAction::ShowSearch,
            ),
            (
                tr("Remote-Verbindungen verwalten …").into(),
                PaletteAction::ShowRemoteDialog,
            ),
            (
                tr("Projekt öffnen …").into(),
                PaletteAction::ShowProfiles(false),
            ),
            (
                tr("Projektprofil speichern …").into(),
                PaletteAction::ShowProfiles(true),
            ),
            (
                tr("Workspace speichern (Strg+Umschalt+S)").into(),
                PaletteAction::SaveWorkspace,
            ),
            (
                tr("Einstellungen öffnen / schließen").into(),
                PaletteAction::ToggleSettings,
            ),
            (
                tr("Hilfe & Tastenkürzel (F1)").into(),
                PaletteAction::ToggleHelp,
            ),
            (
                tr("Über TerminalVelocity & Release Notes").into(),
                PaletteAction::ToggleAbout,
            ),
        ]);
        for preset in [
            effects::Preset::Off,
            effects::Preset::Aurora,
            effects::Preset::Phosphor,
            effects::Preset::DeepSpace,
        ] {
            entries.push((
                trf!("Grafikeffekt-Preset: {preset}", preset = preset.label()),
                PaletteAction::SetPreset(preset),
            ));
        }
        entries
    }

    pub(crate) fn execute_palette_action(&mut self, action: PaletteAction, ctx: &egui::Context) {
        match action {
            PaletteAction::NewTab(shell) => self.add_tab_with_shell(ctx, shell),
            PaletteAction::Split(axis, shell) => self.split_with_shell(ctx, axis, shell),
            PaletteAction::OpenRemote(connection, placement) => {
                if let Err(error) = self.open_remote(connection, placement, ctx) {
                    self.status = error;
                }
            }
            PaletteAction::OpenProfile(index, directory) => {
                if let Err(error) = self.open_profile(index, directory, ctx) {
                    self.status = error;
                }
            }
            PaletteAction::SwitchTab(index) => {
                if index < self.workspace.tabs.len() {
                    self.editor.active = false;
                    self.workspace.active_tab = index;
                    self.clear_tab_notifications(index);
                    self.focus_first();
                }
            }
            PaletteAction::ActivateEditor => {
                self.close_search();
                self.editor.activate();
            }
            PaletteAction::ShowSearch => self.show_search(),
            PaletteAction::ShowRemoteDialog => {
                self.close_search();
                self.remote_dialog = Some(remote::Dialog::default());
            }
            PaletteAction::ShowProfiles(saving) => self.show_profiles(saving),
            PaletteAction::SaveWorkspace => {
                self.save_workspace_explicit();
            }
            PaletteAction::ToggleSettings => {
                self.settings = !self.settings;
            }
            PaletteAction::ToggleHelp => {
                self.help = !self.help;
            }
            PaletteAction::ToggleAbout => {
                self.about = !self.about;
            }
            PaletteAction::SetPreset(preset) => {
                self.workspace.effects.preset = preset;
                self.status = trf!("Grafikeffekt-Preset: {preset}", preset = preset.label());
            }
        }
    }

    pub(crate) fn palette_window(&mut self, ctx: &egui::Context) {
        let Some(mut palette) = self.palette.take() else {
            return;
        };
        let all = self.palette_entries();
        let filtered: Vec<_> = all
            .into_iter()
            .filter(|(label, _)| palette_matches_query(label, &palette.query))
            .collect();
        if filtered.is_empty() {
            palette.selected = 0;
        } else {
            palette.selected = palette.selected.min(filtered.len() - 1);
        }
        let close = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        if close {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) && !filtered.is_empty()
        {
            palette.selected = (palette.selected + 1) % filtered.len();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) && !filtered.is_empty() {
            palette.selected = (palette.selected + filtered.len() - 1) % filtered.len();
        }
        let enter = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        let mut chosen = enter
            .then(|| filtered.get(palette.selected).map(|(_, a)| a.clone()))
            .flatten();
        let mut open = true;
        egui::Window::new(tr("Befehlspalette · Strg+Umschalt+P"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(560.0)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 64.0])
            .show(ctx, |ui| {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut palette.query)
                        .desired_width(f32::INFINITY)
                        .hint_text(tr("Shell, KI, Verbindung, Profil oder Aktion suchen …")),
                );
                if palette.focus_input {
                    response.request_focus();
                    palette.focus_input = false;
                }
                if response.changed() {
                    palette.selected = 0;
                }
                ui.small(tr(
                    "Pfeil hoch/runter: Auswahl · Enter oder Klick: Ausführen · Esc: Schließen",
                ));
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        if filtered.is_empty() {
                            ui.weak(tr("Keine passende Aktion gefunden."));
                        }
                        for (index, (label, action)) in filtered.iter().enumerate() {
                            let item = ui.selectable_label(index == palette.selected, label);
                            if index == palette.selected {
                                item.scroll_to_me(Some(egui::Align::Center));
                            }
                            if item.clicked() {
                                chosen = Some(action.clone());
                            }
                        }
                    });
            });
        if let Some(action) = chosen {
            self.execute_palette_action(action, ctx);
            return;
        }
        if open {
            self.palette = Some(palette);
        }
    }
}
