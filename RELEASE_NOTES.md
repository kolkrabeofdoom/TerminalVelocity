# TerminalVelocity 0.9.3

Security-Hardening- und Wartungs-Release für Windows x64. Alle bestehenden Funktionen bleiben erhalten; zusätzlich wurden sechs Befunde aus dem Sicherheitsreview gehärtet und durch neue Regressionstests abgesichert.

## Sicherheitshärtungen in 0.9.3

- **Link-Öffnung (`Strg+Klick`)**: Beschränkung auf wohlgeformte `http://`- und `https://`-URLs mit gültigem Host; Blockade von `file://`- und Fremdprotokoll-Schemata sowie Beseitigung eines möglichen Prozess-Panics bei fehlerhaften Links.
- **Bracketed-Paste & Pastejacking-Schutz (`Strg+Umschalt+V`)**: Unterstützung von `TermMode::BRACKETED_PASTE` (`\x1b[200~` … `\x1b[201~`) und Filterung eingebetteter `ESC`-/`CSI`-/`NUL`-Sequenzen beim Einfügen aus der Zwischenablage.
- **Schutz vor UNC-/SMB-Netzwerkproben (inkl. Symlinks/Junctions)**: Ablehnung von UNC- (`\\server\share`) und Windows-Gerätepfaden (`\\.\...`) sowie kanonisierter UNC-Ziele bei `TERMINALVELOCITY_CWD:`-Titeln, Rechtsklick-Bild-/Editorpfaden, Hintergrundbildern, SSH-Schlüsseln und benutzerdefinierten Shell-Pfaden.
- **Sichere `PATH`-Reihenfolge**: In PowerShell-Sitzungen behält der System-`PATH` (`Machine` / `System32`) Vorrang vor dem Benutzer-`PATH` (`User`).
- **Validierung von Paket- und Workspace-Pfaden**: Schutz vor Pfad-Traversierung (`..`) beim Auflösen von npm-`package.json`-Startdateien, Ablehnung innerer Anführungszeichen (`"`) in Programm-/Schlüsselpfaden, Validierung von Tab-Namen und Fließkommawerten sowie 2-MiB-Größenlimit beim Laden von `workspace.json` und `profiles.json`.
- **Robuste ConPTY-Fehlerbehandlung**: Rückgabe regulärer Fehlerwerte statt Prozessabbruch (`assert_eq!`) bei `CreatePseudoConsole` / `ResizePseudoConsole`.
- **Atomare Konfigurationsspeicherung**: Speichern von `workspace.json` und `profiles.json` über eindeutige Staging-Dateien mit `create_new(true)`, `sync_all()` und Symlink-Schutz; Entfernung des Debug-Schreibzugriffs `transparency-status.txt`.
- **Antigravity CLI (`agy.exe`) statt Gemini CLI**: Direkte Unterstützung und automatische Erkennung von `agy.exe` (`%LOCALAPPDATA%\agy\bin\agy.exe` / `%USERPROFILE%\.gemini\bin\agy.exe`), einschließlich PowerShell-Alias `agy` und KI-Übergabe per `--prompt-interactive`. Vorhandene `"Gemini"`-Einträge in `workspace.json` und `profiles.json` werden automatisch übernommen.
- **Dateipfade & Stacktraces im Editor öffnen**: Rechtsklick auf lokale Dateipfade oder `pfad:zeile(:spalte)`-Ausgaben im Terminal öffnet die Datei direkt im integrierten Editor und springt zur Zielzeile.
- **Terminalauswahl im Editor öffnen**: Markierter Terminaltext lässt sich per Rechtsklick direkt als ungespeichertes Scratchpad-Dokument im Editor öffnen.
- **Befehlspalette (`Strg+Umschalt+P` / „Befehle …“)**: Schnelles Filtern und Ausführen von Shells/KI-CLIs (Tab oder Split), gespeicherten SSH-/WinRM-Verbindungen, Projektprofilen, offenen Tabs und Grafik-Presets.
- **Hintergrund-Benachrichtigungen (`●`)**: Visuelle Markierung an Hintergrund-Tabs und inaktiven Bereichen bei `BEL`-Signalen oder Prozessende.

## Enthaltene Funktionen

- Tabs und verschachtelte Teilungen mit verschiebbaren Trennlinien, Übernahme lokaler Arbeitsordner und Neustart einzelner Sitzungen.
- PowerShell 7, CMD und optional Git Bash sowie Startmöglichkeiten für separat installierte Codex-, Claude-, Antigravity- und OpenAI-CLIs.
- Textauswahl, Zwischenablage, erkannte Textlinks und wörtliche Suche im Terminalverlauf mit Treffernavigation.
- Bearbeitbare Übergabe markierter Terminalausgabe an eine neue KI-Sitzung, einschließlich vier erklärender Schnellaktionen. Gesendet wird erst nach Bestätigung im Entwurf.
- SSH über OpenSSH und PowerShell-Remoting über WinRM mit gespeicherten Verbindungen und Favoriten.
- Workspace-Wiederherstellung und benannte Projektprofile für Tabs, Programme, Verbindungen und Ordner. Dabei werden neue Prozesse gestartet.
- Lokaler UTF-8-Editor für ein Dokument bis 256 KiB: Syntaxfarben, Suche, Undo/Redo, Einrückung, Konfliktprüfung beim Speichern und Schutz ungespeicherter Änderungen.
- PNG-, JPEG-, WebP- und BMP-Anzeige per Dateiauswahl, Drag-and-drop oder Kontextmenü auf einem erkannten lokalen Bildpfad; Sixel-Ausgabe aus Terminalprogrammen.
- Einstellbare Schriftgröße und Transparenz, optionale Hintergrundbilder mit Abdunklung, Aurora-/Phosphor-/Deep-Space-Effekte, Cursor-Schweif und Text-Nachleuchten.

## Download und Start

`TerminalVelocity-0.9.3-Windows-x64.zip` vollständig in einen beschreibbaren Ordner entpacken und `TerminalVelocity.exe` starten. Bitte auch `conpty.dll`, `OpenConsole.exe` und den Ordner `shells` mitnehmen.

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

Der Release-Stand hat den lokalen Windows-CI-Ablauf bestanden: Formatierung, 116 Tests, Release-Build und vier zusätzliche PTY-Prüfungen für Ordnerübernahme, Neustart, Projektprofile und Verlaufssuche. Die Tests senden keine KI-Anfragen und bauen keine echten Remote-Verbindungen auf. Einzelheiten und die Grenzen der Prüfung stehen im mitgelieferten `VERIFICATION.md`.

Die vollständige Bedienung ist in der README des Release-Tags dokumentiert: https://github.com/kolkrabeofdoom/TerminalVelocity/blob/v0.9.3/README.md
