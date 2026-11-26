# TerminalVelocity 1.0.1 „SecureAllConnections-Fix“

Wartungs- und Sicherheitsrelease für Windows x64. Es schließt vier Lücken, die ein Code-Review nach 1.0.0 gefunden hat: eine Datenverlustfalle beim Laden des Arbeitsbereichs, fälschbare Ordnermeldungen, unvollständige PowerShell-Maskierung und verlorene Ordnermeldungen bei Sonderzeichen. Neue Funktionen gibt es keine, die Bedienung bleibt gleich.

## Behoben in 1.0.1

- **Arbeitsbereich wird nicht mehr still überschrieben.** Konnte `workspace.json` nicht geladen werden (beschädigt, ungültige Verbindung, neuere Dateiversion), startete TerminalVelocity mit Standardwerten und überschrieb die Datei beim Beenden. Gespeicherte SSH-/WinRM-Verbindungen, Programmpfade und Layout waren dann weg. Jetzt wird die Datei zuerst als `workspace.json.broken-<Zeitstempel>` gesichert, automatisches Speichern ist ausgesetzt, und nur **Workspace speichern** (Strg+Umschalt+S) ersetzt sie bewusst. Gelingt die Sicherung nicht, wird gar nicht gespeichert.
- **Ordnermeldungen lassen sich nicht mehr fälschen.** Der Arbeitsordner eines Bereichs kam bisher aus einem Fenstertitel, den jede Ausgabe setzen konnte, etwa eine mit `type` angezeigte Datei, `git log` oder ein von Hand gestartetes `ssh`. Neue Tabs, Teilungen und KI-Sitzungen konnten so in einem fremden Ordner starten; CMD sucht Programme zuerst dort. Jetzt trägt jede Meldung einen zufälligen 128-Bit-Schlüssel der Sitzung, alles andere wird ignoriert.
- **Typografische Anführungszeichen in PowerShell.** PowerShell behandelt `‘ ’ ‚ ‛` wie `'`, maskiert wurde aber nur `'`. Dadurch scheiterte der Start von KI-CLIs bei Pfaden wie `C:\Users\D’Angelo`, und Konto oder Endpunkt einer WinRM-Verbindung konnten den String vorzeitig beenden. Alle vier Varianten werden jetzt verdoppelt.
- **Ordner mit Sonderzeichen in PowerShell.** PowerShell schrieb die Ordnermeldung über die Konsolen-Codepage; Zeichen wie `’`, `ł`, CJK oder Emoji gingen dabei verloren und die Meldung wurde verworfen. Der Pfad wird jetzt UTF-8-hexkodiert übertragen.

## Hinweise zum Update

- `workspace.json` und `profiles.json` aus 1.0.0 können unverändert übernommen werden.
- Eigene Skripte, die selbst `TERMINALVELOCITY_CWD:<pfad>` als Fenstertitel ausgeben, haben keine Wirkung mehr. Das ist beabsichtigt: Nur die von TerminalVelocity gestarteten Shells kennen den Sitzungsschlüssel.
- Wer Git Bash mit einem eigenen Startskript statt `shells/terminalvelocity.bash` nutzt, muss es aus dem Paket übernehmen, damit Ordnerwechsel weiter erkannt werden.

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

`TerminalVelocity-1.0.1-Windows-x64.zip` vollständig in einen beschreibbaren Ordner entpacken und `TerminalVelocity.exe` starten. Bitte auch `conpty.dll`, `OpenConsole.exe` und den Ordner `shells` mitnehmen.

Enthalten sind PowerShell 7.6.7, Microsoft ConPTY 1.25.260930003, das Git-Bash-Startskript, eine Sixel-Demo, Dokumentation und Lizenzdateien. Git Bash, OpenSSH und KI-CLIs sind nicht enthalten und werden bei Bedarf separat benötigt. Persönliche Einstellungen, SSH-Schlüssel und Zugangsdaten sind nicht Bestandteil des Pakets.

Die beigefügte `.sha256`-Datei enthält die ZIP-Prüfsumme. `BUILD-INFO.json` im Paket nennt den Quellcommit, die Toolchain, Laufzeitversionen und die Prüfsummen der enthaltenen Dateien.

## Bekannte Grenzen

- SSH- und WinRM-Verbindungen wurden mit echten Servern praktisch geprüft; die automatisierten Verbindungstests arbeiten weiterhin ohne Remote-Server.
- Native Dateidialoge, Explorer-Ablage, direktes Bildöffnen per Rechtsklick und Grafiktreiberverhalten sind noch nicht vollständig im sichtbaren Fenster geprüft.
- Fenster-, Layout- und Schriftgrößenänderungen löschen Terminalbilder. Sixel ist keine vollständige VT340-Emulation; Kitty-Grafikprotokoll, Bildzoom und Bildexport fehlen.
- Angezeigte Dateipfade werden aus Terminaltext erkannt. Mehrdeutige oder abgeschnittene Namen können eine manuelle Dateiauswahl erfordern. Remote-Pfade werden nicht als lokale Dateien geöffnet.
- Workspace und Projektprofile speichern keine laufenden Prozesse, Ausgaben oder KI-Chats. Der Editor bietet keine Wiederherstellung nach einem erzwungenen Prozessabbruch.

## Prüfung

Der Release-Stand hat den Windows-CI-Ablauf auf GitHub Actions (Windows Server 2022, Rust 1.89.0) bestanden: Formatierung, 121 automatisierte Tests einschließlich der Offline-Integrationstests mit echtem PowerShell-Parser und OpenSSH, Release-Build und vier PTY-Prüfungen für Ordnerübernahme, Neustart, Projektprofile und Verlaufssuche. Die Ordnerprüfung läuft mit PowerShell, CMD und Git Bash in einem Ordner mit Leerzeichen, `'`, `’`, `ł` und `ü` im Namen. `clippy` war für 1.0.1 nicht Teil des Prüflaufs. Die Tests senden keine KI-Anfragen und bauen keine echten Remote-Verbindungen auf. Einzelheiten stehen im mitgelieferten `VERIFICATION.md`.

Die vollständige Bedienung ist in der README des Release-Tags dokumentiert: https://github.com/kolkrabeofdoom/TerminalVelocity/blob/v1.0.1/README.md
