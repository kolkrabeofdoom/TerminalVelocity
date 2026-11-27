//! Recursive drawing of the pane tree.

use crate::*;

pub(crate) struct PaneRenderContext<'a> {
    pub(crate) sessions: &'a mut HashMap<u64, Session>,
    pub(crate) focused: &'a mut u64,
    pub(crate) font_size: f32,
    pub(crate) effects: &'a effects::Settings,
    pub(crate) background_image: Option<egui_term::BackgroundImage>,
    pub(crate) enabled: bool,
    pub(crate) close: &'a mut Option<u64>,
    pub(crate) retry: &'a mut Option<u64>,
    pub(crate) ai_selection: &'a mut Option<ai::Draft>,
    pub(crate) image_request: &'a mut Option<inline_image::Request>,
    pub(crate) editor_request: &'a mut Option<editor::Request>,
}

pub(crate) fn draw_node(
    ui: &mut egui::Ui,
    rect: Rect,
    node: &mut Node,
    ctx: &mut PaneRenderContext<'_>,
) {
    match node {
        Node::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            let columns = matches!(axis, Axis::Columns);
            let extent = if columns { rect.width() } else { rect.height() };
            let offset = extent * *ratio;
            let (mut a, mut b, mut bar) = (rect, rect, rect);
            if columns {
                a.max.x = rect.min.x + offset - 3.0;
                b.min.x = a.max.x + 6.0;
                bar.min.x = a.max.x;
                bar.max.x = b.min.x;
            } else {
                a.max.y = rect.min.y + offset - 3.0;
                b.min.y = a.max.y + 6.0;
                bar.min.y = a.max.y;
                bar.max.y = b.min.y;
            }
            let id = ui.id().with(format!("split-{}", first_id(first)));
            let response = ui.interact(bar, id, Sense::drag());
            if response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    *ratio = ((if columns {
                        pos.x - rect.min.x
                    } else {
                        pos.y - rect.min.y
                    }) / extent.max(1.0))
                    .clamp(0.15, 0.85);
                }
            }
            ui.painter().rect_filled(
                bar,
                0.0,
                if response.hovered() {
                    Color32::from_rgb(70, 180, 190)
                } else {
                    Color32::from_gray(50)
                },
            );
            draw_node(ui, a, first, ctx);
            draw_node(ui, b, second, ctx);
        }
        Node::Pane(pane) => {
            if rect.width() < 40.0 || rect.height() < 70.0 {
                return;
            }
            let enabled = ctx.enabled;
            if enabled
                && ui.input(|i| {
                    i.pointer.any_pressed()
                        && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
                })
            {
                *ctx.focused = pane.id;
            }
            let active = pane.id == *ctx.focused;
            ui.painter().rect_stroke(
                rect.shrink(0.5),
                3.0,
                Stroke::new(
                    1.0_f32,
                    if active {
                        if ctx.effects.visual().strength() > 0.0 {
                            ctx.effects.visual().accent()
                        } else {
                            Color32::from_rgb(66, 195, 185)
                        }
                    } else {
                        Color32::from_gray(55)
                    },
                ),
                egui::StrokeKind::Inside,
            );
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect.shrink(5.0)), |ui| {
                ui.set_clip_rect(rect.shrink(2.0));
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(
                        if active {
                            Color32::from_rgb(100, 220, 200)
                        } else {
                            Color32::GRAY
                        },
                        pane.remote
                            .as_ref()
                            .map(|r| r.label())
                            .unwrap_or_else(|| pane.shell.label().into()),
                    );
                    ui.small(format!("#{}", pane.id));
                    if ctx.sessions.get(&pane.id).is_some_and(|s| s.notified) {
                        ui.colored_label(ctx.effects.visual().accent(), "●")
                            .on_hover_text(tr("Neues Signal oder beendete Sitzung in diesem Bereich"));
                    }
                    if ui
                        .add_enabled(
                            enabled,
                            egui::Button::new(if pane.remote.is_some() {
                                tr("Neu verbinden")
                            } else {
                                tr("Sitzung neu starten")
                            })
                            .small(),
                        )
                        .on_hover_text(
                            tr("Gleicher Anbieter und aktueller Projektordner; bisherige Ausgabe wird geleert"),
                        )
                        .clicked()
                    {
                        *ctx.retry = Some(pane.id);
                    }
                    if ui
                        .add_enabled(enabled, egui::Button::new("×").small())
                        .on_hover_text(tr("Bereich schließen · Strg+Umschalt+W"))
                        .clicked()
                    {
                        *ctx.close = Some(pane.id);
                    }
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!(
                                "{}{}",
                                if pane.remote.is_some() {
                                    tr("Lokaler Startordner: ")
                                } else {
                                    ""
                                },
                                pane.directory.display()
                            ))
                            .small()
                            .weak(),
                        )
                        .truncate(),
                    )
                    .on_hover_text(pane.directory.display().to_string());
                });
                ui.separator();
                if let Some(session) = ctx.sessions.get_mut(&pane.id) {
                    if let Some(error) = &session.error {
                        ui.colored_label(Color32::LIGHT_RED, error);
                        if ui
                            .add_enabled(enabled, egui::Button::new(tr("Sitzung neu starten")))
                            .clicked()
                        {
                            *ctx.retry = Some(pane.id);
                        }
                    }
                    if session.exited {
                        ui.colored_label(
                            Color32::YELLOW,
                            if pane.remote.is_some() {
                                tr("Verbindung beendet. Ausgabe bleibt erhalten; oben mit „Neu verbinden“ erneut öffnen.")
                            } else {
                                tr("Sitzung beendet. Mit „Sitzung neu starten“ oben erneut öffnen.")
                            },
                        );
                    }
                    if let Some(backend) = &mut session.backend {
                        let view = TerminalView::new(ui, backend)
                            .set_background_image(ctx.background_image)
                            .set_effects(ctx.effects.visual())
                            .set_theme(ctx.effects.theme())
                            .set_focus(active && enabled && !session.exited)
                            .set_font(TerminalFont::new(FontSettings {
                                font_type: FontId::monospace(ctx.font_size),
                            }));
                        let response = ui.add(view);
                        if enabled {
                            let over_image_target = ui.input(|i| {
                                i.pointer
                                    .latest_pos()
                                    .is_some_and(|p| response.rect.contains(p))
                            });
                            if over_image_target {
                                let files = inline_image::take_drop(ui.ctx(), response.rect);
                                if !files.is_empty() {
                                    *ctx.image_request =
                                        Some(inline_image::Request::Drop(pane.id, files));
                                }
                                if ui.input(|i| !i.raw.hovered_files.is_empty()) {
                                    ui.painter().rect_stroke(
                                        response.rect.shrink(1.0),
                                        2.0,
                                        Stroke::new(2.0_f32, Color32::LIGHT_BLUE),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                            }
                            let image_target_id = response.id.with("clicked-image-path");
                            let editor_target_id = response.id.with("clicked-editor-path");
                            if response.secondary_clicked() {
                                let (image_path, editor_path) = if pane.remote.is_none() {
                                    if let Some((text, offset)) =
                                        response.interact_pointer_pos().and_then(|pos| {
                                            backend.text_at_pixel(
                                                pos.x - response.rect.min.x,
                                                pos.y - response.rect.min.y,
                                            )
                                        })
                                    {
                                        (
                                            inline_image::path_at(&text, offset, &pane.directory),
                                            editor::path_at(&text, offset, &pane.directory),
                                        )
                                    } else {
                                        (None, None)
                                    }
                                } else {
                                    (None, None)
                                };
                                ui.ctx().data_mut(|data| {
                                    data.insert_temp(image_target_id, image_path);
                                    data.insert_temp(editor_target_id, editor_path);
                                });
                            }
                            response.context_menu(|ui| {
                                let editor_target = ui
                                    .ctx()
                                    .data(|data| {
                                        data.get_temp::<Option<(PathBuf, Option<usize>)>>(
                                            editor_target_id,
                                        )
                                    })
                                    .flatten();
                                let editor_label =
                                    match editor_target.as_ref().and_then(|(_, line)| *line) {
                                        Some(line) => trf!("Datei im Editor öffnen (Zeile {line})", line = line),
                                        None => tr("Datei im Editor öffnen").into(),
                                    };
                                if ui
                                    .add_enabled(
                                        editor_target.is_some(),
                                        egui::Button::new(editor_label),
                                    )
                                    .on_hover_text(
                                        editor_target
                                            .as_ref()
                                            .map(|(p, line)| match line {
                                                Some(l) => format!("{}:{l}", p.display()),
                                                None => p.display().to_string(),
                                            })
                                            .unwrap_or_else(|| {
                                                tr("Rechtsklick auf einen lokalen Dateipfad (optional mit :Zeile).")
                                                    .into()
                                            }),
                                    )
                                    .clicked()
                                {
                                    *ctx.editor_request = editor_target.map(|(path, line)| {
                                        editor::Request::OpenFile(path, line)
                                    });
                                    ui.close_menu();
                                }
                                let path = ui
                                    .ctx()
                                    .data(|data| data.get_temp::<Option<PathBuf>>(image_target_id))
                                    .flatten();
                                if ui
                                    .add_enabled(path.is_some(), egui::Button::new(tr("Bild öffnen")))
                                    .on_hover_text(
                                        path.as_ref()
                                            .map(|p| p.display().to_string())
                                            .unwrap_or_else(|| {
                                                tr("Rechtsklick auf einen lokalen Bilddateipfad. Remote-Pfade werden nicht lokal geöffnet.")
                                                    .into()
                                            }),
                                    )
                                    .clicked()
                                {
                                    *ctx.image_request =
                                        path.map(|path| inline_image::Request::Open(pane.id, path));
                                    ui.close_menu();
                                }
                                if ui.button(tr("Bild auswählen …")).clicked() {
                                    *ctx.image_request =
                                        Some(inline_image::Request::Choose(pane.id));
                                    ui.close_menu();
                                }
                                ui.separator();
                                let text = backend.selectable_content();
                                let selected = !text.trim().is_empty();
                                if ui.add_enabled(selected, egui::Button::new(tr("Kopieren"))).clicked() {
                                    ui.ctx().copy_text(text.clone());
                                    ui.close_menu();
                                }
                                if ui
                                    .add_enabled(
                                        selected,
                                        egui::Button::new(tr("Auswahl im Editor öffnen")),
                                    )
                                    .clicked()
                                {
                                    *ctx.editor_request =
                                        Some(editor::Request::OpenSelection(text.clone()));
                                    ui.close_menu();
                                }
                                if ui
                                    .add_enabled(selected, egui::Button::new(tr("An KI übergeben …")))
                                    .clicked()
                                {
                                    *ctx.ai_selection = Some(ai::Draft::new(
                                        text.clone(),
                                        pane.directory.clone(),
                                        pane.shell,
                                    ));
                                    ui.close_menu();
                                }
                                ui.separator();
                                if pane.remote.is_some() {
                                    ui.small(tr("KI-Übergabe öffnet eine lokale KI-Sitzung."));
                                }
                                for action in ai::QuickAction::ALL {
                                    if ui
                                        .add_enabled(selected, egui::Button::new(action.label()))
                                        .clicked()
                                    {
                                        *ctx.ai_selection = Some(ai::Draft::new(
                                            action.prompt(&text),
                                            pane.directory.clone(),
                                            pane.shell,
                                        ));
                                        ui.close_menu();
                                    }
                                }
                                if !selected {
                                    ui.weak(tr("Zuerst Text im Terminal markieren."));
                                }
                            });
                        }
                    }
                }
            });
        }
    }
}
