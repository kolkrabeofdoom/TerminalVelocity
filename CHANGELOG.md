# Änderungen

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
