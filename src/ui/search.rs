//! History search bar (Ctrl+Shift+F).

use crate::*;

pub(crate) struct HistorySearch {
    pub(crate) pane_id: u64,
    pub(crate) query: String,
    pub(crate) focus_input: bool,
    pub(crate) error: Option<String>,
}

impl TerminalApp {
    pub(crate) fn close_search(&mut self) {
        if let Some(search) = self.search.take() {
            if let Some(backend) = self
                .sessions
                .get_mut(&search.pane_id)
                .and_then(|s| s.backend.as_mut())
            {
                let _ = backend.set_search("");
            }
        }
    }

    pub(crate) fn show_search(&mut self) {
        if self.editor.active {
            self.editor.start_search();
            return;
        }
        if let Some(search) = &mut self.search {
            search.focus_input = true;
        } else if self
            .sessions
            .get(&self.focused)
            .is_some_and(|s| s.backend.is_some())
        {
            self.search = Some(HistorySearch {
                pane_id: self.focused,
                query: String::new(),
                focus_input: true,
                error: None,
            });
        } else {
            self.status = tr("In diesem Bereich ist kein Terminalverlauf verfügbar.").into();
        }
    }

    pub(crate) fn search_bar(&mut self, ctx: &egui::Context, background: Color32) {
        if self
            .search
            .as_ref()
            .is_some_and(|s| s.pane_id != self.focused)
            || self.has_modal_dialog()
        {
            self.close_search();
        }
        let Some(search) = &mut self.search else {
            return;
        };
        let Some(backend) = self
            .sessions
            .get_mut(&search.pane_id)
            .and_then(|s| s.backend.as_mut())
        else {
            self.search = None;
            return;
        };
        let mut close = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        let mut previous = ctx.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Enter));
        let mut next = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        egui::TopBottomPanel::bottom("history_search")
            .frame(egui::Frame::NONE.fill(background).inner_margin(6.0))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(tr("Verlauf durchsuchen"));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut search.query)
                            .id_salt("history-query")
                            .desired_width(280.0)
                            .hint_text(tr("Suchtext (Groß-/Kleinschreibung beachten)")),
                    );
                    if search.focus_input {
                        response.request_focus();
                        search.focus_input = false;
                    }
                    if response.changed() {
                        search.error = backend.set_search(&search.query).err();
                    }
                    let (active, count, truncated) = backend.search_status();
                    ui.label(if search.query.is_empty() {
                        tr("Suchtext eingeben").into()
                    } else if count == 0 {
                        tr("Keine Treffer").into()
                    } else {
                        format!("{active} / {count}{}", if truncated { "+" } else { "" })
                    });
                    previous |= ui
                        .add_enabled(count > 0, egui::Button::new(tr("Zurück")))
                        .on_hover_text("Umschalt+Enter")
                        .clicked();
                    next |= ui
                        .add_enabled(count > 0, egui::Button::new(tr("Weiter")))
                        .on_hover_text("Enter")
                        .clicked();
                    close |= ui.button(tr("Schließen (Esc)")).clicked();
                    if let Some(error) = &search.error {
                        ui.colored_label(Color32::LIGHT_RED, error);
                    }
                });
            });
        if previous || next {
            backend.next_search_match(previous);
            search.focus_input = true;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(300));
        if close {
            self.close_search();
        }
    }
}
