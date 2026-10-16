# Rust Terminal

Windows-Terminal mit Tabs, teilbaren Bereichen und integrierten Startprofilen für KI-CLIs. Stand: **0.2.0**.

## Funktionen

- PowerShell 7, CMD und optional Git Bash.
- Codex CLI, Claude Code, Gemini CLI und OpenAI API CLI als eigene Terminalbereiche.
- Neue Tabs und Teilungen übernehmen den aktuellen Projektordner des aktiven Bereichs. PowerShell, CMD und Git Bash melden Verzeichniswechsel an jedem Prompt.
- **Sitzung neu starten** direkt im Bereich: gleicher Anbieter und Projektordner, frischer Prozess und leere Ausgabe. Laufende Sitzungen erfordern vorher eine Bestätigung.
- Markierten Text per Kontextmenü an eine KI übergeben: Anbieter, Projektordner und Text vor dem Senden bearbeiten; die KI öffnet in einem neuen Tab.
- KI-Schnellaktionen im Kontextmenü: **Fehler erklären**, **Lösung vorschlagen**, **Befehl erklären** und **Ausgabe zusammenfassen** bereiten einen bearbeitbaren Entwurf vor.
- Benannte Projektprofile speichern Tabs, Teilungen, Anbieter und Ordner als Vorlage und öffnen sie als neue Sitzungen in zusätzlichen Tabs.
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

Das Skript baut das Projekt, lädt die in `shells/powershell-source.json` festgeschriebene offizielle PowerShell-Version herunter, prüft SHA256 und erstellt `dist/RustTerminal-0.2.0-Windows-x64.zip`. Mit `-SkipBuild` lässt sich ein bereits gebautes Release verpacken. Bereits installierte KI-CLIs werden automatisch gesucht; sie werden nicht in das Paket kopiert.

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

## Markierten Text an eine KI übergeben

1. Text im Terminal mit der Maus markieren. In Programmen mit eigener Maussteuerung dabei **Umschalt** gedrückt halten.
2. Rechtsklick auf den Terminalbereich → **An KI übergeben …** oder eine der vier Schnellaktionen auswählen.
3. Anbieter und Projektordner prüfen; im Dialog eine Frage ergänzen oder den Text bearbeiten.
4. **Senden und KI-Tab öffnen** anklicken.

Die Schnellaktionen ergänzen eine passende deutsche Anweisung und übernehmen die Auswahl einschließlich Zeilenumbrüchen. Ohne Markierung sind die Aktionen deaktiviert. Der gesamte Entwurf bleibt vor dem Senden bearbeitbar; das Limit von 10.000 Windows-Zeicheneinheiten umfasst auch die ergänzte Anweisung. Die Prompts fordern ausschließlich eine Erklärung oder einen Vorschlag an, ohne Befehle auszuführen oder Dateien zu ändern. Das ist eine Anweisung an die Anbieter-CLI, keine technische Einschränkung ihrer Werkzeuge; deren Berechtigungen und Bestätigungen gelten weiterhin.

Unterstützt werden Codex, Claude, Gemini und OpenAI API CLI. Letztere benötigt eine eigene Modell-ID und einen bereits eingerichteten API-Schlüssel. Der Dialog öffnet eine neue Sitzung; bestehende KI-Chats werden nicht beschrieben. **Abbrechen** schließt den Entwurf ohne Übergabe. Der Entwurf wird nicht in `workspace.json` gespeichert. Der Text ist auf 10.000 Windows-Zeicheneinheiten begrenzt, damit er in die Befehlszeile passt.

Die Übergabe erfolgt als Programmargumente, ohne den Text als Shell-Befehl auszuführen. npm-Starter werden dafür direkt über Node.js und ihren Programmeinstieg aufgerufen. Mehrzeilige Auswahlen behalten Zeilenumbrüche und berücksichtigen den Scrollback.

## Projektprofile

1. Projektordner, Anbieter, Tabs und Teilungen wie gewünscht einrichten.
2. **Projekt speichern …** wählen, einen eindeutigen Namen und den vollständigen Projektordner eintragen, dann **Profil speichern** anklicken. Die Vorlage umfasst alle aktuell geöffneten Tabs und Bereiche sowie den Standardanbieter für neue Bereiche.
3. Später **Projekt öffnen** wählen, das Profil auswählen und die Bereichsordner in der Vorschau prüfen.
4. Bei Bedarf den Projektordner ändern. Bereiche innerhalb des ursprünglichen Projektordners ziehen einschließlich Unterordnern mit um; externe Ordner behalten ihren absoluten Pfad.
5. **Projekt in neuen Tabs öffnen** startet neue Sitzungen. Bestehende Tabs und Prozesse bleiben erhalten. Fehlende Ordner oder Anbieterprogramme werden vor dem Öffnen gemeldet; es gelten insgesamt maximal 32 Tabs und 64 Bereiche.

Unter **Projekt öffnen → Profil löschen …** kann eine Vorlage nach Bestätigung entfernt werden. Zum Speichern einer anderen Zusammenstellung einen neuen Namen verwenden oder die alte Vorlage vorher löschen. Maximal 32 Profile mit Namen von 1 bis 80 Zeichen sind möglich.

Profile liegen lokal in `profiles.json` neben der EXE; fehlerhafte Dateien werden gemeldet und nicht automatisch überschrieben. Darstellungsoptionen und CLI-Programmpfade bleiben globale Einstellungen. Profile speichern ausschließlich die Startkonfiguration, keine Prozesse, Terminalausgaben, Anmeldedaten, KI-Entwürfe oder Chat-Verläufe. Die vorhandene `workspace.json` bleibt kompatibel und speichert weiterhin den zuletzt benutzten Arbeitsbereich. Beide lokalen Dateien sind vom Repository ausgeschlossen.

## Prüfung

`cargo test --locked` prüft unter anderem Workspace-Kompatibilität, Ordnerübernahme bei Fokuswechsel, Teilungen, Anbieterpfade und den nativen Codex-Start. Die gebaute EXE bietet weitere lokale Tests:

```powershell
./dist/RustTerminal/RustTerminal.exe --smoke-test
./dist/RustTerminal/RustTerminal.exe --directory-test
./dist/RustTerminal/RustTerminal.exe --restart-test
./dist/RustTerminal/RustTerminal.exe --profile-test
```

Dabei werden Berichte neben der EXE erzeugt. Die Verzeichnis- und Neustarttests verwenden echte Windows-Pseudoterminals. Details und bekannte Grenzen stehen in [VERIFICATION.md](VERIFICATION.md).

Für den Offline-Übergabetest zunächst `cargo build --release --locked --features test-tools` ausführen. Die erzeugte `prompt-probe.exe` neben die Terminal-EXE kopieren und diese mit `--handoff-test` starten. Der Test verwendet echte PTYs und ein lokales Konsolen-Testprogramm; es werden keine KI-Anfragen gesendet.

## Technik und Lizenzen

Rust, eframe/egui 0.31, alacritty_terminal 0.25 und die angepasste egui_term 0.1.0. Alle Rust-Abhängigkeiten sind über `Cargo.lock` festgeschrieben.

Projektlizenz: [MIT](LICENSE). Änderungen an der eingebetteten Terminalbibliothek sind in [VENDOR_CHANGES.md](VENDOR_CHANGES.md) dokumentiert; deren ursprüngliche Lizenz bleibt erhalten. Weitere Abhängigkeitslizenzen sind in `THIRD_PARTY_LICENSES.txt` enthalten. PowerShell behält beim Verpacken seine mitgelieferten Lizenzdateien.

