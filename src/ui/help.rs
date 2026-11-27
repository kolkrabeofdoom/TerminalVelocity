//! Help (F1) and About windows.

use crate::*;

impl TerminalApp {
    pub(crate) fn help_window(&mut self, ctx: &egui::Context) {
        if !self.help {
            return;
        }
        let mut open = self.help;
        egui::Window::new(tr("Hilfe · TerminalVelocity"))
            .open(&mut open)
            .default_width(640.0)
            .max_height(520.0)
            .vscroll(true)
            .show(ctx, |ui| {
                ui.heading(tr("Tastenkürzel"));
                ui.add_space(4.0);
                egui::Grid::new("help-shortcuts")
                    .striped(true)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        for (shortcut, desc) in [
                            ("F1", tr("Diese Hilfe öffnen / schließen")),
                            ("Strg+Umschalt+P", tr("Befehlspalette öffnen")),
                            ("Strg+Umschalt+T", tr("Neuen Tab öffnen")),
                            ("Strg+Umschalt+D / E", tr("Bereich nebeneinander / untereinander teilen")),
                            ("Strg+Tab / Strg+Umschalt+Rechts", tr("Nächster Bereich / nächster Tab")),
                            ("Strg+Umschalt+W", tr("Aktiven Bereich schließen")),
                            ("Strg+Umschalt+S", tr("Workspace speichern (im Editor: Speichern unter)")),
                            ("Strg+Umschalt+C / V", tr("Auswahl kopieren / sicher einfügen (Bracketed Paste)")),
                            ("Strg+F", tr("Terminalverlauf bzw. Dokument durchsuchen")),
                            ("Strg+Klick", tr("Erkannten HTTP(S)-Link im Browser öffnen")),
                        ] {
                            ui.monospace(shortcut);
                            ui.label(desc);
                            ui.end_row();
                        }
                    });
                ui.separator();
                ui.heading(tr("Bedienung & Funktionen"));
                ui.add_space(4.0);
                for (title, body) in [
                    (
                        tr("Dateipfade, Stacktraces & Auswahl im Editor"),
                        tr("Rechtsklick auf einen lokalen Dateipfad oder eine Compiler-/Fehlermeldung (z. B. src\\main.rs:142:5) → „Datei im Editor öffnen (Zeile X)“. Markierter Text lässt sich per Rechtsklick als ungespeichertes Dokument in den Editor übernehmen."),
                    ),
                    (
                        tr("KI-Übergabe & Shells"),
                        tr("PowerShell 7, CMD, Git Bash sowie Codex, Claude, Antigravity (agy.exe) und OpenAI CLI lassen sich oben oder über die Befehlspalette starten. Markierter Text kann über das Kontextmenü geprüft an eine neue KI-Sitzung übergeben werden."),
                    ),
                    (
                        tr("Remote & Projektprofile"),
                        tr("Unter „Verbinden …“ lassen sich SSH- und PowerShell-WinRM-Ziele speichern. „Projekt speichern …“ sichert komplette Tab- und Split-Layouts inkl. relativer Unterordner als Vorlage."),
                    ),
                    (
                        tr("Bilder & Benachrichtigungen"),
                        tr("PNG, JPEG, WebP und BMP lassen sich per Drag-and-drop, Dateiauswahl oder Rechtsklick sowie über Sixel direkt im Terminal anzeigen. Ein farbiger Punkt (●) an Tabs/Bereichen meldet Glockensignale oder beendete Hintergrundsitzungen."),
                    ),
                ] {
                    ui.strong(title);
                    ui.label(body);
                    ui.add_space(4.0);
                }
            });
        self.help = open;
    }

    pub(crate) fn about_window(&mut self, ctx: &egui::Context) {
        if !self.about {
            return;
        }
        if self.about_banner.is_none() {
            if let Ok(decoded) = image::load_from_memory(include_bytes!("../../assets/banner.jpg"))
            {
                let rgba = decoded.to_rgba8();
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                );
                self.about_banner = Some(ctx.load_texture(
                    "about-banner",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
        let mut open = self.about;
        egui::Window::new(tr("Über TerminalVelocity"))
            .open(&mut open)
            .default_width(640.0)
            .max_height(580.0)
            .show(ctx, |ui| {
                if let Some(banner) = &self.about_banner {
                    let size = banner.size_vec2();
                    let width = 320.0_f32;
                    let height = (width * size.y / size.x.max(1.0)).round();
                    ui.vertical_centered(|ui| {
                        ui.add(
                            egui::Image::new((banner.id(), egui::vec2(width, height)))
                                .corner_radius(6.0),
                        );
                    });
                    ui.add_space(6.0);
                }
                ui.horizontal(|ui| {
                    ui.heading("TerminalVelocity");
                    ui.label(
                        egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .strong()
                            .color(Color32::from_rgb(100, 210, 190)),
                    );
                });
                ui.label(tr("Portables Windows-Terminal mit Split-Bereichen, KI-CLI-Integration, SSH/WinRM, integriertem UTF-8-Editor und Sixel-/Bildanzeige."));
                ui.small("MIT-Lizenz · Rust + egui/eframe + ConPTY");
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label("GitHub-Repository:");
                    ui.hyperlink_to(
                        "https://github.com/kolkrabeofdoom/TerminalVelocity",
                        "https://github.com/kolkrabeofdoom/TerminalVelocity",
                    );
                    ui.separator();
                    ui.hyperlink_to(
                        "Releases & Downloads",
                        "https://github.com/kolkrabeofdoom/TerminalVelocity/releases",
                    );
                });
                ui.separator();
                ui.strong(tr("Aktuelle Release Notes"));
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(include_str!("../../RELEASE_NOTES.md"))
                                .size(13.0),
                        );
                    });
            });
        self.about = open;
    }
}
