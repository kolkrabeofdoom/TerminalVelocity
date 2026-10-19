# Lokale Anpassungen an egui_term 0.1.0

Quelle: https://crates.io/crates/egui_term/0.1.0, Repository: https://github.com/Harzu/egui_term

- `src/view.rs`: Tastatureingaben erreichen den fokussierten Terminalbereich auch dann, wenn der Mauszeiger außerhalb liegt. Mausereignisse bleiben auf den Bereich unter dem Zeiger begrenzt.
- `src/view.rs`: Der Standardhintergrund des Terminal-Widgets ist transparent; die Anwendung zeichnet die einstellbare Hintergrunddeckkraft. Explizite ANSI-Hintergrundfarben bleiben erhalten.
- `src/backend/mod.rs`: Der Ereignis-Thread endet bei geschlossenen Kanälen ohne Warteschleife oder Panic.
- `src/backend/mod.rs`: Programmpfade und Argumente werden unter Windows getrennt und korrekt maskiert an ConPTY übergeben. Auswahltexte werden dadurch nicht zu Shell-Befehlen.
- `src/backend/mod.rs`: Die Textauswahl verwendet `selection_to_string`, einschließlich Zeilenumbrüchen, umgebrochener Zeilen und Scrollback.
- `src/lib.rs`: `SelectionType` ist für den Offline-Auswahltest exportiert.
- `src/view.rs`: Umschalt+Ziehen erlaubt Textauswahl auch in Programmen mit eigener Maussteuerung.
- `src/backend/mod.rs`: Wörtliche, case-sensitive Verlaufssuche mit Alacrittys Grid-Suchiterator; weiche Zeilenumbrüche, Navigation mit Umlauf und Scrollen zum aktuellen Treffer. Die Trefferliste wird während der Suche alle 300 ms aktualisiert und ist auf 10.000 Treffer begrenzt.
- `src/view.rs`: Treffer werden gelb bzw. orange hervorgehoben. Die Zellzuordnung verwendet binäre Suche; die normale Textauswahl hat Vorrang.

Die ursprüngliche MIT-Lizenz liegt unverändert unter `vendor/egui_term-0.1.0/LICENSE`.



## 0.4.0: Optionale Darstellungseffekte

- Neue VisualEffects-Konfiguration mit Aurora-Mesh, Phosphor-Scanlines und Sternpartikeln.
- Cursor-Schweif und Konturleuchten vor dem Zeichnen der Terminalzellen; auf den Bereich begrenzt, mit Ablaufzeit und Rücksetzen bei Fokus-/Layout-/Scrolländerungen.
- TerminalView::set_effects; Anpassung berücksichtigt SHOW_CURSOR und sichtbaren Scrollbereich. Animationen werden nur bei fokussiertem aktiven Terminal angefordert.
- Tests für Trail-Grenzen und Lebenszyklus. Keine Änderung an PTY-Protokoll oder Eingabeformat.


## 0.5.0: Hintergrundtexturen

BackgroundImage und TerminalView::set_background_image zeichnen eine geteilte GPU-Textur vor Effekten und Terminalzellen. Seitenverhältnis bleibt bei mittigem Zuschnitt oder Einpassen erhalten; Abdunklung und Clipping gelten je Terminalbereich. Der Decoder und asynchrone Ladecache liegen im Hauptprogramm.

## 0.6.0: Text-Nachleuchten

TextAfterglow speichert maximal 8192 aktuelle Glyphen und 512 Leuchtzeitpunkte. Neue/geänderte sichtbare Glyphen erhalten vier versetzte, quadratisch abklingende Lichtkonturen vor den Terminalzellen. Gelöschte Glyphen werden sofort verworfen. Fokus, Animationseinstellungen, Auswahl/Suchtreffer und Ansichtsänderungen begrenzen den Effekt. Die Tests der unveränderten Renderer-Quelldatei laufen jetzt über tests/renderer.rs in der Haupt-Suite.
