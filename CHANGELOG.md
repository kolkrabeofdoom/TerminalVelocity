# Änderungen

## Unveröffentlicht

- **Consolas auch ohne `C:`**: Die Terminalschrift wird aus `%SystemRoot%\Fonts` geladen statt fest aus `C:\Windows\Fonts`. Liegt Windows auf einem anderen Laufwerk, fiel TerminalVelocity bisher still auf die eingebaute Schrift zurück.
- **Kein Hängen bei getrennten Netzlaufwerken**: Meldet eine Shell einen Ordner auf einem eingehängten Netzlaufwerk, prüft TerminalVelocity dessen Existenz nicht mehr im Fenster-Thread; geprüft wird beim Start einer Sitzung dort. Meldungen ohne Ordnerwechsel greifen gar nicht mehr aufs Dateisystem zu.
- Intern: Testmodi nur noch mit Feature `test-tools`, `main.rs` in Module zerlegt, `clippy` in der CI.

## 1.0.1 – „SecureAllConnections-Fix“ – 26. November 2026

- **Kein stiller Verlust von `workspace.json`**: Lässt sich die Datei nicht laden, wird sie als `workspace.json.broken-<Zeitstempel>` gesichert. Automatisches Speichern (Beenden, Einstellungen, Verbindungsdialog) ist ausgesetzt; nur „Workspace speichern“ ersetzt die Datei bewusst, und nur wenn die Sicherung gelungen ist. Bisher wurden gespeicherte Verbindungen, Programmpfade und Layout beim Beenden mit Standardwerten überschrieben.
- **Ordnermeldungen mit Sitzungsschlüssel**: Den Arbeitsordner eines Bereichs übernimmt TerminalVelocity nur noch aus Meldungen mit einem zufälligen 128-Bit-Schlüssel der Sitzung (`TERMINALVELOCITY_CWD:<schlüssel>:<pfad>`). Angezeigte Dateien, `git log` oder von Hand gestartete SSH-Sitzungen können neue Tabs, Teilungen und KI-Sitzungen nicht mehr in fremde Ordner umleiten. Git Bash erhält den Schlüssel per Umgebungsvariable, die das Startskript sofort entfernt.
- **Typografische Anführungszeichen in PowerShell**: `‘ ’ ‚ ‛` werden wie `'` maskiert. Behebt den fehlschlagenden KI-CLI-Start bei Pfaden wie `C:\Users\D’Angelo` und verhindert, dass Konto oder Endpunkt einer WinRM-Verbindung den PowerShell-String vorzeitig beenden.
- **PowerShell-Ordnermeldung unabhängig von der Codepage**: PowerShell meldet den Pfad UTF-8-hexkodiert (`TERMINALVELOCITY_CWDX:`). Ordner mit Zeichen außerhalb der Konsolen-Codepage (`’`, `ł`, CJK, Emoji …) werden jetzt korrekt übernommen statt still verworfen.
- **CI**: Die Berichte der PTY-Tests stehen im Log; der Ordner-Test läuft mit `’` und `ł` im Pfad für PowerShell, CMD und Git Bash.
- `egui_term`: `BackendSettings::env` reicht Umgebungsvariablen an ConPTY durch (siehe `VENDOR_CHANGES.md`).

## 1.0.0 – Voll-Release

- **Hilfe-Sektion (`F1` / „Hilfe“)**: Integrierte Übersicht aller Tastenkürzel und Bedienungsabläufe direkt in der Anwendung.
- **Über-Bereich („Über“)**: Zeigt Version, Lizenz, direkte Links zum GitHub-Repository und den Releases sowie die aktuellen Release Notes (`RELEASE_NOTES.md`).
- **Code-Review- & Editor-Verbesserungen**: Rückwärtssuche (`Umschalt+Enter`) und `Escape` zum Schließen in der Editor-Suche, erweitertes Syntax-Highlighting (inkl. Rust, Python, JS/TS, Go, C/C++, YAML mit `//`-Kommentaren), entkoppelter `PaneRenderContext` und vollständige Beseitigung aller `clippy`-Warnungen (`-D warnings`).
- **README-Banner**: Neues TerminalVelocity-Banner (`assets/banner.jpg`) und vollständige 1.0.0-Dokumentation.

## 0.9.3 – Security-Hardening-Release

- **Link-Härtung (`Strg+Klick`)**: Nur noch wohlgeformte `http://`- und `https://`-Links mit nicht-leerem Host werden erkannt und geöffnet; `file://`-, UNC- und Fremdprotokoll-Schemata sowie der Prozess-Panic bei fehlgeschlagenem `open::that`-Aufruf wurden entfernt.
- **Bracketed-Paste & Pastejacking-Schutz (`Strg+Umschalt+V`)**: Beim Einfügen wird `TermMode::BRACKETED_PASTE` (`\x1b[200~` … `\x1b[201~`) unterstützt und eingebettete `ESC`-/`CSI`-/`NUL`-Steuerzeichen werden neutralisiert, sodass mehrzeilige Texte in modernen Shells nicht unbeabsichtigt sofort ausgeführt werden.
- **Schutz vor SMB/UNC-Netzwerkzugriffen (inkl. Symlinks/Junctions)**: UNC- (`\\server\share`, `//server/share`, `\\?\UNC\...`) und Windows-Gerätepfade (`\\.\...`) sowie lokale Symlinks/Junctions auf UNC-Ziele werden vor Dateisystemzugriffen abgewiesen.
- **Sichere PowerShell-`PATH`-Reihenfolge**: Der System-`PATH` (`Machine` / `System32`) behält in gestarteten PowerShell-Sitzungen Vorrang vor dem Benutzer-`PATH` (`User`).
- **Pfad-, Quotierungs- & Konfigurationsvalidierung**: Schutz vor Pfad-Traversierung (`..`) in `node_entry`, Ablehnung innerer Anführungszeichen (`"`) in Programm-/Schlüsselpfaden, Längen-/Steuerzeichenprüfung für Tab-Namen, NaN-sichere Fließkomma-Validierung sowie 2-MiB-Größenbegrenzung beim Laden von `workspace.json`.
- **ConPTY-Fehlerbehandlung**: Fehler bei `CreatePseudoConsole` und `ResizePseudoConsole` werden sauber als `io::Error` bzw. Warnung verarbeitet statt die Anwendung per `assert_eq!` zu beenden.
- **Atomares Speichern & Bereinigung**: `workspace.json` und `profiles.json` werden über eindeutige Staging-Dateien (`create_new(true)`, `sync_all()`, Symlink-Prüfung) atomar ersetzt; der Debug-Schreibzugriff auf `transparency-status.txt` wurde entfernt.
- **Antigravity CLI (`agy.exe`)**: Ersetzt den bisherigen Gemini-CLI-Eintrag durch die native Antigravity CLI (`agy.exe`) inklusive Auto-Erkennung unter `%LOCALAPPDATA%\agy\bin` und `%USERPROFILE%\.gemini\bin`, PowerShell-Alias `agy`, KI-Übergabe über `--prompt-interactive` und automatischer Migration bestehender `"Gemini"`-Konfigurationseinträge.
- **Lokale Dateipfade & Stacktraces direkt im Editor öffnen**: Rechtsklick auf Dateipfade oder Compiler-/Stacktrace-Einträge (`pfad:zeile` / `pfad:zeile:spalte`) in der Terminalausgabe öffnet die lokale Textdatei direkt im integrierten Editor und springt zur angegebenen Zeile.
- **Auswahl im Editor öffnen**: Rechtsklick auf markierten Terminaltext übernimmt die Auswahl als neues, ungespeichertes Dokument (Scratchpad) in den integrierten Editor.
- **Befehlspalette (`Strg+Umschalt+P` / „Befehle …“)**: Zentrale Schnellauswahl mit Stichwortfilter für Shells, KI-CLIs (Tab oder Split), gespeicherte SSH-/WinRM-Verbindungen, Projektprofile, offene Tabs, Editor, Suche, Einstellungen und Grafik-Presets.
- **Hintergrund-Aktivitätsanzeige (`●`)**: Farbiger Indikator in der Tableiste und Bereichsüberschrift, wenn ein nicht fokussierter Bereich ein Glockensignal (`BEL`) sendet oder beendet wird.

## 0.9.2 – 22. Oktober 2026

- Erstes veröffentlichtes GitHub-Release mit portablem Windows-x64-Paket und SHA256-Datei.
- README vollständig nach Aufgaben neu gegliedert: Installation, sämtliche vorhandenen Funktionen, Bedienung, Datenhaltung, Grenzen, Build und Prüfung.
- Eigene Release Notes, auch im Download-Paket enthalten. Keine funktionalen Änderungen gegenüber 0.9.1.

## 0.9.1 – 22. Oktober 2026

- „Bild öffnen“ im Kontextmenü verwendet den angeklickten lokalen Bildpfad direkt, ohne Windows-Dateidialog.
- „Bild auswählen …“ bleibt eine separate Aktion mit Dateiauswahl.
- Erkennung relativer/absoluter Pfade, Dateinamen mit Leerzeichen/Unicode und umgebrochener Terminalzeilen; der beim Rechtsklick ermittelte Pfad bleibt während der Menübedienung fest.
- Remote-Bereiche öffnen angeklickte Pfade nicht als lokale Dateien. Relative Pfade gelten zum aktuellen lokalen Sitzungsordner.

## 0.9.0 – 21. Oktober 2026

- „Bild anzeigen …“ im Terminal-Kontextmenü und Drag-and-drop einer lokalen Bilddatei auf den gewünschten Bereich.
- PNG, JPEG, WebP und BMP werden direkt als RGBA dargestellt, ohne Sixel-Konverter oder Farbpalettenreduktion.
- Laden im Hintergrund, proportionale Verkleinerung auf die Terminalbreite und bestehende Speichergrenzen; keine Vergrößerung kleiner Bilder.
- Gemeinsamer Bildspeicher für lokale Bilder und Sixel, einschließlich Scrollback, Löschen und Texturfreigabe. Die Einstellung zum Abschalten gilt für beide Eingangswege.
- Ladeergebnisse werden bei geschlossener/neugestarteter Zielsitzung oder geänderter Bereichsbreite verworfen.

## 0.8.0 – 20. Oktober 2026

- Sixel-Bilder im Terminal: 256 Farbregister, RGB/HLS, Wiederholungen, Rastermaße, Pixelseitenverhältnis und Transparenz.
- Zeilenverankerte Darstellung mit Scrollback, Teil-Löschung durch Text, getrennten Haupt-/Alternativbildschirmen und begrenztem Bildspeicher.
- Abfragen der Grafikfähigkeit und Zellgröße, DEC-Modi 80/1070; PTY-Antworten werden an das laufende Programm zurückgeleitet.
- Abschaltbare Unterstützung in den Einstellungen und mitgeliefertes lokales Demo-Skript.
- Portable Microsoft-ConPTY-Laufzeit mit festgeschriebener Version und SHA256, damit Bildsequenzen auf Windows ankommen.
- Größen-/Schriftänderungen löschen Bilder; Einfügen/Löschen von Zeichen entfernt betroffene Bildsegmente. Keine dauerhafte Bildspeicherung.

## 0.7.0 – 20. Oktober 2026

- Schlanker lokaler Editor als eigener Tab: Öffnen, Neu, Speichern/Speichern unter, Zeilennummern, Einrückung, Undo/Redo und Suche.
- Einfache Syntaxfarben für PowerShell, Bash, JSON, TOML und Markdown, ohne zusätzliche Laufzeit.
- UTF-8/BOM und LF/CRLF erhalten; Konfliktprüfung vor atomarem Speichern und Bestätigung für ungespeicherte Änderungen, auch beim Beenden.
- Begrenzung auf 256 KiB und ein Dokument zur Zeit; keine Ausführung oder Remote-Bearbeitung.

## 0.6.0 – 19. Oktober 2026

- Einstellbares Text-Nachleuchten für neue und geänderte Zeichen im Phosphor-Modus, mit Leuchtstärke und Dauer.
- Sofortiges Entfernen gelöschter Zeichen; begrenzter Renderaufwand und Rücksetzen bei Ansichtsänderungen.
- Renderer-Prüfungen laufen jetzt direkt in der Haupt-Test-Suite und in CI.

## 0.5.1 – 19. Oktober 2026

- Einheitlicher Projektname TerminalVelocity in Dokumentation, EXE, Paketordner, Build- und CI-Skripten sowie Shell-Integration. Das Cargo-Paket heißt `terminalvelocity`, die Anwendung `TerminalVelocity.exe`.
- Das Git-Bash-Startskript heißt jetzt `shells/terminalvelocity.bash`; interne Prompt-Kennungen verwenden `TERMINALVELOCITY_CWD`.
- Bestehende `workspace.json` und `profiles.json` bleiben lesbar. Beim Umzug in einen neuen Programmordner diese beiden Dateien übernehmen; relative Hintergrundbild- und Programmpfade beziehen sich weiterhin auf den Programmordner.
- Automatische Windows-Prüfungen für Pushes und Pull Requests.

## 0.5.0 – 19. Oktober 2026

Erster gemeinsam gesicherter Release-Stand nach 0.2.1.

- SSH und PowerShell über WinRM mit gespeicherten Verbindungen, Favoriten, Tabs und Teilungen; Remote-Verbindungen sind in Projektprofilen enthalten.
- Lokale Projektordner bleiben von entfernten Terminalausgaben getrennt. KI-Übergaben sind als lokal gekennzeichnet.
- Aurora, Phosphor und Deep Space, Cursor-Schweif und Konturleuchten; Intensität und Animationen einstellbar.
- Eigene Hintergrundbilder (PNG, JPEG, WebP, BMP) mit asynchronem Laden, Abdunklung, mittigem Zuschnitt oder Einpassen.
- Git-Bash-Startskript wird auch beim normalen Build neben die EXE kopiert.
- Release-Paket mit festgeschriebener PowerShell-Version, SHA256-Prüfung, BUILD-INFO.json und deterministischer ZIP-Erstellung aus gleichen Eingabedateien.

Validierung: automatisierte Rust- und Offline-Integrationstests sowie bestehende Windows-PTY-Tests. Die lokale Oberfläche wurde von der nutzenden Person als gut aussehend bestätigt. Eine vollständige manuelle Prüfung aller Bedienwege und echte SSH-/WinRM-Anmeldungen stehen weiterhin aus. Keine Verbindungstests mit produktiven Servern.

Das ZIP enthält PowerShell 7, aber keine KI-CLIs, SSH-Schlüssel, Kontodaten, persönlichen Bilder oder Workspace-/Verbindungsprofile. OpenSSH muss für SSH-Verbindungen auf dem Zielcomputer vorhanden sein.
