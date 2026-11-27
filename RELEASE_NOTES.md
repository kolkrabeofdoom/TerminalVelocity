# TerminalVelocity 1.1.1 „Kiene Kästkes“

Kleines Folgerelease zu 1.1.0. Zeichen, die KI-CLIs wie Claude Code oder Codex ständig ausgeben, erscheinen nicht mehr als Kästchen.

## Behoben in 1.1.1

- **Ersatzschriften für fehlende Zeichen.** Consolas und die eingebauten Schriften kennen Symbole wie ✓, ✗, ⎿ oder ✻ nicht, ebenso wenig chinesische, japanische und koreanische Schrift. Statt Kästchen springen jetzt Windows-Schriften ein: Segoe UI Symbol, Segoe UI Emoji (schwarz-weiß), Microsoft YaHei und Malgun Gothic aus `%SystemRoot%\Fonts`. Fehlt eine davon, wird sie übersprungen. Der Arbeitsspeicherbedarf steigt dadurch um etwa 25 MB.

## Hinweise zum Update

- `workspace.json` und `profiles.json` aus 1.0.x und 1.1.0 können unverändert übernommen werden.

## Enthaltene Funktionen

- Tabs und verschachtelte Teilungen mit verschiebbaren Trennlinien, Übernahme lokaler Arbeitsordner und Neustart einzelner Sitzungen.
- PowerShell 7, CMD und optional Git Bash sowie Startmöglichkeiten für separat installierte Codex-, Claude-, Antigravity- und OpenAI-CLIs.
- Textauswahl, Zwischenablage, erkannte Textlinks und wörtliche Suche im Terminalverlauf mit Treffernavigation.
- Bearbeitbare Übergabe markierter Terminalausgabe an eine neue KI-Sitzung, einschließlich vier erklärender Schnellaktionen. Gesendet wird erst nach Bestätigung im Entwurf.
- SSH über OpenSSH und PowerShell-Remoting über WinRM mit gespeicherten Verbindungen und Favoriten.
- Workspace-Wiederherstellung und benannte Projektprofile für Tabs, Programme, Verbindungen und Ordner. Dabei werden neue Prozesse gestartet.
- Lokaler UTF-8-Editor für ein Dokument bis 256 KiB: Syntaxfarben, Vor-/Rückwärtssuche, Undo/Redo, Einrückung, Konfliktprüfung beim Speichern und Schutz ungespeicherter Änderungen.
- PNG-, JPEG-, WebP- und BMP-Anzeige per Dateiauswahl, Drag-and-drop oder Kontextmenü auf einem erkannten lokalen Bildpfad; Sixel-Ausgabe aus Terminalprogrammen.
- Oberfläche wahlweise auf Hochdeutsch oder Plattdütsk (Ostwestfaolen), umschaltbar ohne Neustart.
- Ersatzschriften aus Windows für Symbole, Emoji sowie chinesische, japanische und koreanische Schrift.
- Einstellbare Schriftgröße und Transparenz, optionale Hintergrundbilder mit Abdunklung, Aurora-/Phosphor-/Deep-Space-Effekte, Cursor-Schweif und Text-Nachleuchten.

## Download und Start

`TerminalVelocity-1.1.1-Windows-x64.zip` vollständig in einen beschreibbaren Ordner entpacken und `TerminalVelocity.exe` starten. Bitte auch `conpty.dll`, `OpenConsole.exe` und den Ordner `shells` mitnehmen.

Enthalten sind PowerShell 7.6.7, Microsoft ConPTY 1.25.260930003, das Git-Bash-Startskript, eine Sixel-Demo, Dokumentation und Lizenzdateien. Git Bash, OpenSSH und KI-CLIs sind nicht enthalten und werden bei Bedarf separat benötigt. Persönliche Einstellungen, SSH-Schlüssel und Zugangsdaten sind nicht Bestandteil des Pakets.

Die beigefügte `.sha256`-Datei enthält die ZIP-Prüfsumme. `BUILD-INFO.json` im Paket nennt den Quellcommit, die Toolchain, Laufzeitversionen und die Prüfsummen der enthaltenen Dateien.

## Bekannte Grenzen

- SSH- und WinRM-Verbindungen wurden mit echten Servern praktisch geprüft; die automatisierten Verbindungstests arbeiten weiterhin ohne Remote-Server.
- Native Dateidialoge, Explorer-Ablage, direktes Bildöffnen per Rechtsklick und Grafiktreiberverhalten sind noch nicht vollständig im sichtbaren Fenster geprüft.
- Emoji erscheinen schwarz-weiß; farbige Emoji kann der Renderer nicht darstellen.
- Fenster-, Layout- und Schriftgrößenänderungen löschen Terminalbilder. Sixel ist keine vollständige VT340-Emulation; Kitty-Grafikprotokoll, Bildzoom und Bildexport fehlen.
- Angezeigte Dateipfade werden aus Terminaltext erkannt. Mehrdeutige oder abgeschnittene Namen können eine manuelle Dateiauswahl erfordern. Remote-Pfade werden nicht als lokale Dateien geöffnet.
- Die plattdeutsche Übersetzung folgt ravensbergisch-lippischer Schreibung; Platt ist nicht genormt, Korrekturen sind willkommen. Release Notes und README gibt es nur auf Hochdeutsch.
- Workspace und Projektprofile speichern keine laufenden Prozesse, Ausgaben oder KI-Chats. Der Editor bietet keine Wiederherstellung nach einem erzwungenen Prozessabbruch.

## Prüfung

Der Release-Stand hat den Windows-CI-Ablauf auf GitHub Actions (Windows Server 2022, Rust 1.89.0) bestanden: Formatierung, `clippy` mit und ohne Feature `test-tools` ohne Warnungen, Abgleich des Vendor-Codes mit den crates.io-Originalen, 130 automatisierte Tests, Release-Build, vier PTY-Prüfungen und die Prüfung, dass die ausgelieferte EXE keine Testmodi enthält. Neu ist ein Test mit den echten Windows-Schriften: Die eingebauten Schriften haben ✓, ✗, ⎿, ✻, 中, あ und 한 nicht, mit den Ersatzschriften sind sie vorhanden. Einzelheiten stehen im mitgelieferten `VERIFICATION.md`.

Die vollständige Bedienung ist in der README des Release-Tags dokumentiert: https://github.com/kolkrabeofdoom/TerminalVelocity/blob/v1.1.1/README.md
