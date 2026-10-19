# Änderungen

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
