# Rust Terminal

Windows-Terminal mit Tabs, teilbaren Bereichen und integrierten Startprofilen für KI-CLIs. Stand: **0.1.7**.

## Funktionen

- PowerShell 7, CMD und optional Git Bash.
- Codex CLI, Claude Code, Gemini CLI und OpenAI API CLI als eigene Terminalbereiche.
- Neue Tabs und Teilungen übernehmen den aktuellen Projektordner des aktiven Bereichs. PowerShell, CMD und Git Bash melden Verzeichniswechsel an jedem Prompt.
- **Sitzung neu starten** direkt im Bereich: gleicher Anbieter und Projektordner, frischer Prozess und leere Ausgabe. Laufende Sitzungen erfordern vorher eine Bestätigung.
- Windows-Fenstertransparenz mit einstellbarer Deckkraft. Im standardmäßig aktiven Kompatibilitätsmodus werden auch Schrift und Bedienelemente durchsichtig. Alternativ Hintergrundtransparenz mit optionalem Acryl, abhängig vom Grafiktreiber.
- Layout, Anbieter, zuletzt gemeldete Ordner und Darstellung werden lokal in `workspace.json` neben der EXE gespeichert.

Die Anwendung startet vorhandene Anbieter-CLIs. Anmeldung, Abrechnung und Bestätigungen erfolgen beim jeweiligen Anbieter. Vorhandene Terminalausgaben werden nicht automatisch an einen Anbieter gesendet. Ein Neustart öffnet eine neue CLI-Sitzung; er setzt keinen KI-Chat automatisch fort.

## Bauen

Voraussetzungen: Windows x64, Rust Stable sowie entweder Microsoft C++ Build Tools mit Windows SDK (MSVC-Target) oder eine passende GNU-Linker-Toolchain.

```powershell
cargo test --locked
cargo build --release --locked
```

Alternativ: `./scripts/build.ps1`. Das ausführbare Programm liegt anschließend unter `target/release/rust-terminal.exe`.

PowerShell 7 muss im PATH, unter `C:\Program Files\PowerShell\7` oder portabel unter `shells/powershell/pwsh.exe` neben der EXE liegen. Die ältere Windows PowerShell 5.1 wird nicht als Ersatz verwendet. Unter **Einstellungen → Shells und KI-CLIs** können EXE- und CMD-Pfade explizit angegeben werden; relative Pfade gelten zum Programmordner.

## Portables Paket erstellen

```powershell
./scripts/package.ps1
```

Das Skript baut das Projekt, lädt die in `shells/powershell-source.json` festgeschriebene offizielle PowerShell-Version herunter, prüft SHA256 und erstellt `dist/RustTerminal-0.1.7-Windows-x64.zip`. Mit `-SkipBuild` lässt sich ein bereits gebautes Release verpacken. Bereits installierte KI-CLIs werden automatisch gesucht; sie werden nicht in das Paket kopiert.

Dieses Repository enthält die Quellen und die angepasste Terminalbibliothek. Kompilate, heruntergeladene Laufzeiten, Zugangsdaten, lokale Einstellungen und Testausgaben sind ausgeschlossen.

## Bedienung

| Aktion | Tastenkürzel |
| --- | --- |
| Neuer Tab | Strg+Umschalt+T |
| Bereiche nebeneinander | Strg+Umschalt+D |
| Bereiche untereinander | Strg+Umschalt+E |
| Nächster Bereich | Strg+Tab |
| Nächster Tab | Strg+Umschalt+Pfeil rechts |
| Bereich schließen | Strg+Umschalt+W |
| Workspace speichern | Strg+Umschalt+S |
| Kopieren / Einfügen | Strg+Umschalt+C / V |

Für einen KI-Tab zuerst im aktiven Terminal mit `cd` zum Projekt wechseln, oben den Anbieter auswählen und **+ Tab** anklicken. Die Bereichsüberschrift zeigt den zuletzt gemeldeten Ordner. Bei KI-Sitzungen ohne Shell-Prompt bleibt der geerbte Startordner maßgeblich.

**Sitzung neu starten** steht in jedem Bereichskopf. Andere Bereiche laufen weiter. Ein fehlender Projektordner wird als Startfehler angezeigt und nicht stillschweigend ersetzt.

## Prüfung

`cargo test --locked` prüft unter anderem Workspace-Kompatibilität, Ordnerübernahme bei Fokuswechsel, Teilungen, Anbieterpfade und den nativen Codex-Start. Die gebaute EXE bietet weitere lokale Tests:

```powershell
./dist/RustTerminal/RustTerminal.exe --smoke-test
./dist/RustTerminal/RustTerminal.exe --directory-test
./dist/RustTerminal/RustTerminal.exe --restart-test
```

Dabei werden Berichte neben der EXE erzeugt. Die Verzeichnis- und Neustarttests verwenden echte Windows-Pseudoterminals. Details und bekannte Grenzen stehen in [VERIFICATION.md](VERIFICATION.md).

## Technik und Lizenzen

Rust, eframe/egui 0.31, alacritty_terminal 0.25 und die angepasste egui_term 0.1.0. Alle Rust-Abhängigkeiten sind über `Cargo.lock` festgeschrieben.

Projektlizenz: [MIT](LICENSE). Änderungen an der eingebetteten Terminalbibliothek sind in [VENDOR_CHANGES.md](VENDOR_CHANGES.md) dokumentiert; deren ursprüngliche Lizenz bleibt erhalten. Weitere Abhängigkeitslizenzen sind in `THIRD_PARTY_LICENSES.txt` enthalten. PowerShell behält beim Verpacken seine mitgelieferten Lizenzdateien.

