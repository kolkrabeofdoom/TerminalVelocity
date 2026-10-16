# Lokale Anpassungen an egui_term 0.1.0

Quelle: https://crates.io/crates/egui_term/0.1.0, Repository: https://github.com/Harzu/egui_term

- `src/view.rs`: Tastatureingaben erreichen den fokussierten Terminalbereich auch dann, wenn der Mauszeiger außerhalb liegt. Mausereignisse bleiben auf den Bereich unter dem Zeiger begrenzt.
- `src/view.rs`: Der Standardhintergrund des Terminal-Widgets ist transparent; die Anwendung zeichnet die einstellbare Hintergrunddeckkraft. Explizite ANSI-Hintergrundfarben bleiben erhalten.
- `src/backend/mod.rs`: Der Ereignis-Thread endet bei geschlossenen Kanälen ohne Warteschleife oder Panic.
- `src/backend/mod.rs`: Programmpfade und Argumente werden unter Windows getrennt und korrekt maskiert an ConPTY übergeben. Auswahltexte werden dadurch nicht zu Shell-Befehlen.
- `src/backend/mod.rs`: Die Textauswahl verwendet `selection_to_string`, einschließlich Zeilenumbrüchen, umgebrochener Zeilen und Scrollback.
- `src/lib.rs`: `SelectionType` ist für den Offline-Auswahltest exportiert.
- `src/view.rs`: Umschalt+Ziehen erlaubt Textauswahl auch in Programmen mit eigener Maussteuerung.

Die ursprüngliche MIT-Lizenz liegt unverändert unter `vendor/egui_term-0.1.0/LICENSE`.

