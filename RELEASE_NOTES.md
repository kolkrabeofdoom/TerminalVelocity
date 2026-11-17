# TerminalVelocity 1.0.0

Erstes stabiles Voll-Release für Windows x64. TerminalVelocity vereint ein schnelles, portables Windows-Terminal mit geteilten Ansichten, nahtloser KI-CLI-Integration (inkl. Antigravity CLI), SSH-/WinRM-Verbindungen, einem integrierten UTF-8-Editor, Sixel-/Bildanzeige, Befehlspalette und umfassender Sicherheitshärtung.

## Neu in 1.0.0

- **Hilfe-Sektion (`F1` / „Hilfe“)**: Interaktive Übersicht aller Tastenkürzel, Bedienungsabläufe und Funktionen direkt in der App.
- **Über-Bereich („Über“)**: Zeigt Version, Lizenz, direkte Links zum [GitHub-Repository](https://github.com/kolkrabeofdoom/TerminalVelocity) und zur Release-Seite sowie die aktuellen Release Notes.
- **Erweiterter Editor & Komfortfunktionen**:
  - Rechtsklick auf lokale Dateipfade oder Compiler-/Stacktrace-Ausgaben (`pfad:zeile(:spalte)`) öffnet die Datei direkt im Editor und springt zur Zielzeile.
  - Markierter Terminaltext lässt sich per Rechtsklick als ungespeichertes Scratchpad-Dokument im Editor öffnen.
  - Editor-Suchleiste mit Vor-/Rückwärtsnavigation (`Enter` / `Umschalt+Enter`), `Escape` zum Schließen und erweitertem Syntax-Highlighting (inkl. PowerShell, Bash, JSON, TOML, Markdown, Rust, Python, TypeScript/JavaScript, Go, C/C++ und YAML).
- **Befehlspalette (`Strg+Umschalt+P` / „Befehle …“)**: Schnelles Filtern und Ausführen von Shells/KI-CLIs (Tab oder Split), gespeicherten SSH-/WinRM-Verbindungen, Projektprofilen, offenen Tabs, Hilfe/Über und Grafik-Presets.
- **Hintergrund-Benachrichtigungen (`●`)**: Farbige Aktivitätsmarkierung an Hintergrund-Tabs und inaktiven Bereichen bei `BEL`-Signalen oder Prozessende.
- **Antigravity CLI (`agy.exe`)**: Direkte Unterstützung und automatische Erkennung von `agy.exe` inklusive PowerShell-Alias `agy`, KI-Übergabe per `--prompt-interactive` und automatischer Migration alter `"Gemini"`-Einträge.

## Sicherheitshärtungen (TV-01 bis TV-11)

- **Bracketed-Paste & Pastejacking-Schutz (`Strg+Umschalt+V`)**: Unterstützung von `TermMode::BRACKETED_PASTE` (`\x1b[200~` … `\x1b[201~`) und Filterung eingebetteter `ESC`-/`CSI`-/`NUL`-Sequenzen beim Einfügen aus der Zwischenablage.
- **Link-Öffnung (`Strg+Klick`)**: Beschränkung auf wohlgeformte `http://`- und `https://`-URLs mit gültigem Host; Blockade von `file://`- und Fremdprotokoll-Schemata ohne Panic bei fehlerhaften Links.
- **Schutz vor UNC-/SMB-Netzwerkproben (inkl. Symlinks/Junctions)**: Ablehnung von UNC- (`\\server\share`) und Windows-Gerätepfaden (`\\.\...`) sowie kanonisierter UNC-Ziele bei `TERMINALVELOCITY_CWD:`-Titeln, Rechtsklick-Bild-/Editorpfaden, Hintergrundbildern, SSH-Schlüsseln und benutzerdefinierten Shell-Pfaden.
- **Sichere `PATH`-Reihenfolge**: In PowerShell-Sitzungen behält der System-`PATH` (`Machine` / `System32`) Vorrang vor dem Benutzer-`PATH` (`User`).
- **Validierung & atomare Speicherung**: Schutz vor Pfad-Traversierung (`..`) in `package.json`, Ablehnung innerer Anführungszeichen (`"`) in Programm-/Schlüsselpfaden, Validierung von Tab-Namen und nicht-finiten Fließkommawerten, 2-MiB-Größenlimit beim Laden von `workspace.json`/`profiles.json` sowie atomare Speicherung über eindeutige Staging-Dateien.

## Enthaltene Funktionen

- Tabs und verschachtelte Teilungen mit verschiebbaren Trennlinien, Übernahme lokaler Arbeitsordner und Neustart einzelner Sitzungen.
- PowerShell 7, CMD und optional Git Bash sowie Startmöglichkeiten für separat installierte Codex-, Claude-, Antigravity- und OpenAI-CLIs.
- Textauswahl, Zwischenablage, erkannte Textlinks und wörtliche Suche im Terminalverlauf mit Treffernavigation.
- Bearbeitbare Übergabe markierter Terminalausgabe an eine neue KI-Sitzung, einschließlich vier erklärender Schnellaktionen. Gesendet wird erst nach Bestätigung im Entwurf.
- SSH über OpenSSH und PowerShell-Remoting über WinRM mit gespeicherten Verbindungen und Favoriten.
- Workspace-Wiederherstellung und benannte Projektprofile für Tabs, Programme, Verbindungen und Ordner. Dabei werden neue Prozesse gestartet.
- Lokaler UTF-8-Editor für ein Dokument bis 256 KiB: Syntaxfarben, Vor-/Rückwärtssuche, Undo/Redo, Einrückung, Konfliktprüfung beim Speichern und Schutz ungespeicherter Änderungen.
- PNG-, JPEG-, WebP- und BMP-Anzeige per Dateiauswahl, Drag-and-drop oder Kontextmenü auf einem erkannten lokalen Bildpfad; Sixel-Ausgabe aus Terminalprogrammen.
- Einstellbare Schriftgröße und Transparenz, optionale Hintergrundbilder mit Abdunklung, Aurora-/Phosphor-/Deep-Space-Effekte, Cursor-Schweif und Text-Nachleuchten.

## Download und Start

`TerminalVelocity-1.0.0-Windows-x64.zip` vollständig in einen beschreibbaren Ordner entpacken und `TerminalVelocity.exe` starten. Bitte auch `conpty.dll`, `OpenConsole.exe` und den Ordner `shells` mitnehmen.

Enthalten sind PowerShell 7.6.7, Microsoft ConPTY 1.25.260930003, das Git-Bash-Startskript, eine Sixel-Demo, Dokumentation und Lizenzdateien. Git Bash, OpenSSH und KI-CLIs sind nicht enthalten und werden bei Bedarf separat benötigt. Persönliche Einstellungen, SSH-Schlüssel und Zugangsdaten sind nicht Bestandteil des Pakets.

Für die Übernahme eines vorhandenen Arbeitsbereichs bei geschlossener Anwendung `workspace.json` und gegebenenfalls `profiles.json` aus dem alten Programmordner kopieren. Relative Programm- und Hintergrundbildpfade anschließend prüfen.

Die beigefügte `.sha256`-Datei enthält die ZIP-Prüfsumme. `BUILD-INFO.json` im Paket nennt den Quellcommit, die Toolchain, Laufzeitversionen und die Prüfsummen der enthaltenen Dateien.

## Bekannte Grenzen

- Echte SSH-/WinRM-Anmeldungen sind bisher nicht geprüft. Die automatisierten Verbindungstests arbeiten ohne Remote-Server.
- Native Dateidialoge, Explorer-Ablage, direktes Bildöffnen per Rechtsklick und Grafiktreiberverhalten sind noch nicht vollständig im sichtbaren Fenster geprüft.
- Fenster-, Layout- und Schriftgrößenänderungen löschen Terminalbilder. Sixel ist keine vollständige VT340-Emulation; Kitty-Grafikprotokoll, Bildzoom und Bildexport fehlen.
- Angezeigte Dateipfade werden aus Terminaltext erkannt. Mehrdeutige oder abgeschnittene Namen können eine manuelle Dateiauswahl erfordern. Remote-Pfade werden nicht als lokale Dateien geöffnet.
- Workspace und Projektprofile speichern keine laufenden Prozesse, Ausgaben oder KI-Chats. Der Editor bietet keine Wiederherstellung nach einem erzwungenen Prozessabbruch.

## Prüfung

Der Release-Stand hat den lokalen Windows-CI-Ablauf bestanden: Formatierung, `cargo clippy --all-targets --locked -- -D warnings`, 116 automatisierte Tests, Release-Build und vier zusätzliche PTY-Prüfungen für Ordnerübernahme, Neustart, Projektprofile und Verlaufssuche. Die Tests senden keine KI-Anfragen und bauen keine echten Remote-Verbindungen auf. Einzelheiten und die Grenzen der Prüfung stehen im mitgelieferten `VERIFICATION.md`.

Die vollständige Bedienung ist in der README des Release-Tags dokumentiert: https://github.com/kolkrabeofdoom/TerminalVelocity/blob/v1.0.0/README.md
