# TerminalVelocity 1.1.0 „Nigge Tied“

Funktions- und Wartungsrelease für Windows x64. Neu ist eine zweite Oberflächensprache: Plattdütsk aus Ostwestfalen. Dazu kommen drei Fehlerbehebungen, ein Sicherheitsupdate einer Abhängigkeit und ein gründlicher Umbau hinter den Kulissen, der künftige Änderungen sicherer macht.

## Neu in 1.1.0

- **Plattdütsk (Ostwestfaolen).** Unter **Einstellungen → Sprache** lässt sich die Oberfläche von Hochdeutsch auf ostwestfälisches Platt umstellen, sofort und ohne Neustart. Übersetzt sind Menüs, Dialoge, Status- und Fehlermeldungen und die Hilfe, rund 330 Texte. Die KI-Schnellaktionen bitten die KI dann um eine Antwort auf Plattdeutsch.

## Behoben in 1.1.0

- **Strg+U löscht wieder die Eingabezeile.** Die Tastenkombination schickte ein `Q` statt des Steuerzeichens `0x15`. In Git Bash und anderen Readline-Shells erschien deshalb ein Q, statt die Zeile zu leeren.
- **Consolas auch ohne Laufwerk C:.** Die Terminalschrift wird jetzt aus `%SystemRoot%\Fonts` geladen. Lag Windows auf einem anderen Laufwerk, fiel TerminalVelocity bisher still auf die eingebaute Schrift zurück.
- **Kein Hängen bei getrennten Netzlaufwerken.** Meldet eine Shell einen Ordner auf einem eingehängten Netzlaufwerk, prüft TerminalVelocity ihn nicht mehr im Fenster-Thread. War der Server nicht erreichbar, konnte das Fenster mehrere Sekunden einfrieren. Prompts ohne Ordnerwechsel greifen gar nicht mehr aufs Dateisystem zu.
- **Sicherheitsupdate:** `url` 2.5.9 behebt RUSTSEC-2026-0592 (Stack Overflow bei verschachtelten `blob:`-URLs). Außerdem `clipboard-win` 5.4.3 statt der zurückgezogenen 5.4.1.

## Unter der Haube

- Die Testmodi stecken nicht mehr in der ausgelieferten EXE; sie beendet sich bei `--*-test` mit Exit-Code 2.
- Der Programmcode ist in übersichtliche Module aufgeteilt. Die Bedienung ist dabei unverändert geblieben.
- `clippy` prüft jeden Stand ohne Warnungen. Der angepasste Code der drei mitgelieferten Bibliotheken wird bei jedem Lauf gegen die Originale von crates.io plus dokumentierte Patches geprüft. `cargo deny` sucht wöchentlich nach Sicherheitsmeldungen zu allen Abhängigkeiten.

## Hinweise zum Update

- `workspace.json` und `profiles.json` aus 1.0.x können unverändert übernommen werden. Ohne Spracheinstellung startet TerminalVelocity auf Hochdeutsch.

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
- Einstellbare Schriftgröße und Transparenz, optionale Hintergrundbilder mit Abdunklung, Aurora-/Phosphor-/Deep-Space-Effekte, Cursor-Schweif und Text-Nachleuchten.

## Download und Start

`TerminalVelocity-1.1.0-Windows-x64.zip` vollständig in einen beschreibbaren Ordner entpacken und `TerminalVelocity.exe` starten. Bitte auch `conpty.dll`, `OpenConsole.exe` und den Ordner `shells` mitnehmen.

Enthalten sind PowerShell 7.6.7, Microsoft ConPTY 1.25.260930003, das Git-Bash-Startskript, eine Sixel-Demo, Dokumentation und Lizenzdateien. Git Bash, OpenSSH und KI-CLIs sind nicht enthalten und werden bei Bedarf separat benötigt. Persönliche Einstellungen, SSH-Schlüssel und Zugangsdaten sind nicht Bestandteil des Pakets.

Die beigefügte `.sha256`-Datei enthält die ZIP-Prüfsumme. `BUILD-INFO.json` im Paket nennt den Quellcommit, die Toolchain, Laufzeitversionen und die Prüfsummen der enthaltenen Dateien.

## Bekannte Grenzen

- SSH- und WinRM-Verbindungen wurden mit echten Servern praktisch geprüft; die automatisierten Verbindungstests arbeiten weiterhin ohne Remote-Server.
- Native Dateidialoge, Explorer-Ablage, direktes Bildöffnen per Rechtsklick und Grafiktreiberverhalten sind noch nicht vollständig im sichtbaren Fenster geprüft.
- Fenster-, Layout- und Schriftgrößenänderungen löschen Terminalbilder. Sixel ist keine vollständige VT340-Emulation; Kitty-Grafikprotokoll, Bildzoom und Bildexport fehlen.
- Angezeigte Dateipfade werden aus Terminaltext erkannt. Mehrdeutige oder abgeschnittene Namen können eine manuelle Dateiauswahl erfordern. Remote-Pfade werden nicht als lokale Dateien geöffnet.
- Die plattdeutsche Übersetzung folgt ravensbergisch-lippischer Schreibung; Platt ist nicht genormt, Korrekturen sind willkommen. Release Notes und README gibt es nur auf Hochdeutsch.
- Workspace und Projektprofile speichern keine laufenden Prozesse, Ausgaben oder KI-Chats. Der Editor bietet keine Wiederherstellung nach einem erzwungenen Prozessabbruch.

## Prüfung

Der Release-Stand hat den Windows-CI-Ablauf auf GitHub Actions (Windows Server 2022, Rust 1.89.0) bestanden: Formatierung, `clippy` mit und ohne Feature `test-tools` ohne Warnungen, Abgleich des Vendor-Codes mit den crates.io-Originalen, 127 automatisierte Tests einschließlich der Offline-Integrationstests mit echtem PowerShell-Parser und OpenSSH sowie der Vollständigkeitsprüfung der Übersetzung, Release-Build, vier PTY-Prüfungen für Ordnerübernahme, Neustart, Projektprofile und Verlaufssuche und die Prüfung, dass die ausgelieferte EXE keine Testmodi enthält. `cargo deny` meldet keine Sicherheitslücken. Die Tests senden keine KI-Anfragen und bauen keine echten Remote-Verbindungen auf. Einzelheiten stehen im mitgelieferten `VERIFICATION.md`.

Die vollständige Bedienung ist in der README des Release-Tags dokumentiert: https://github.com/kolkrabeofdoom/TerminalVelocity/blob/v1.1.0/README.md
