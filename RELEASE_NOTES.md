# TerminalVelocity 0.9.2

Erstes veröffentlichtes GitHub-Release für Windows x64. Es bündelt die bisher entwickelten Funktionen in einem portablen Paket. Die vorherigen Versionsstände waren als Git-Tags verfügbar. Gegenüber 0.9.1 wurde die README vollständig neu gegliedert und um eine zusammenhängende Bedienungs-, Installations- und Build-Dokumentation ergänzt; es gibt keine neue Terminalfunktion in diesem Versionsschritt.

## Enthaltene Funktionen

- Tabs und verschachtelte Teilungen mit verschiebbaren Trennlinien, Übernahme lokaler Arbeitsordner und Neustart einzelner Sitzungen.
- PowerShell 7, CMD und optional Git Bash sowie Startmöglichkeiten für separat installierte Codex-, Claude-, Gemini- und OpenAI-CLIs.
- Textauswahl, Zwischenablage, erkannte Textlinks und wörtliche Suche im Terminalverlauf mit Treffernavigation.
- Bearbeitbare Übergabe markierter Terminalausgabe an eine neue KI-Sitzung, einschließlich vier erklärender Schnellaktionen. Gesendet wird erst nach Bestätigung im Entwurf.
- SSH über OpenSSH und PowerShell-Remoting über WinRM mit gespeicherten Verbindungen und Favoriten.
- Workspace-Wiederherstellung und benannte Projektprofile für Tabs, Programme, Verbindungen und Ordner. Dabei werden neue Prozesse gestartet.
- Lokaler UTF-8-Editor für ein Dokument bis 256 KiB: Syntaxfarben, Suche, Undo/Redo, Einrückung, Konfliktprüfung beim Speichern und Schutz ungespeicherter Änderungen.
- PNG-, JPEG-, WebP- und BMP-Anzeige per Dateiauswahl, Drag-and-drop oder Kontextmenü auf einem erkannten lokalen Bildpfad; Sixel-Ausgabe aus Terminalprogrammen.
- Einstellbare Schriftgröße und Transparenz, optionale Hintergrundbilder mit Abdunklung, Aurora-/Phosphor-/Deep-Space-Effekte, Cursor-Schweif und Text-Nachleuchten.

## Download und Start

`TerminalVelocity-0.9.2-Windows-x64.zip` vollständig in einen beschreibbaren Ordner entpacken und `TerminalVelocity.exe` starten. Bitte auch `conpty.dll`, `OpenConsole.exe` und den Ordner `shells` mitnehmen.

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

Der Release-Stand hat den lokalen Windows-CI-Ablauf bestanden: Formatierung, 112 Tests, Release-Build und vier zusätzliche PTY-Prüfungen für Ordnerübernahme, Neustart, Projektprofile und Verlaufssuche. Die Tests senden keine KI-Anfragen und bauen keine echten Remote-Verbindungen auf. Einzelheiten und die Grenzen der Prüfung stehen im mitgelieferten `VERIFICATION.md`.

Die vollständige Bedienung ist in der README des Release-Tags dokumentiert: https://github.com/kolkrabeofdoom/TerminalVelocity/blob/v0.9.2/README.md
