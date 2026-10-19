# Rust Terminal

Windows-Terminal mit Tabs, teilbaren Bereichen und integrierten Startprofilen für KI-CLIs. Stand: **0.5.0**.

## Funktionen

- PowerShell 7, CMD und optional Git Bash.
- Codex CLI, Claude Code, Gemini CLI und OpenAI API CLI als eigene Terminalbereiche.
- Neue Tabs und Teilungen übernehmen den aktuellen Projektordner des aktiven Bereichs. PowerShell, CMD und Git Bash melden Verzeichniswechsel an jedem Prompt.
- **Sitzung neu starten** direkt im Bereich: gleicher Anbieter und Projektordner, frischer Prozess und leere Ausgabe. Laufende Sitzungen erfordern vorher eine Bestätigung.
- Markierten Text per Kontextmenü an eine KI übergeben: Anbieter, Projektordner und Text vor dem Senden bearbeiten; die KI öffnet in einem neuen Tab.
- KI-Schnellaktionen im Kontextmenü: **Fehler erklären**, **Lösung vorschlagen**, **Befehl erklären** und **Ausgabe zusammenfassen** bereiten einen bearbeitbaren Entwurf vor.
- Benannte Projektprofile speichern Tabs, Teilungen, Anbieter und Ordner als Vorlage und öffnen sie als neue Sitzungen in zusätzlichen Tabs.
- Wörtliche Suche im Terminalverlauf mit **Strg+F**, hervorgehobenen Treffern und Vor-/Rückwärtssuche.
- Windows-Fenstertransparenz mit einstellbarer Deckkraft. Im standardmäßig aktiven Kompatibilitätsmodus werden auch Schrift und Bedienelemente durchsichtig. Alternativ Hintergrundtransparenz mit optionalem Acryl, abhängig vom Grafiktreiber.
- Layout, Anbieter, zuletzt gemeldete Ordner und Darstellung werden lokal in `workspace.json` neben der EXE gespeichert.

Die Anwendung startet vorhandene Anbieter-CLIs. Anmeldung, Abrechnung und Bestätigungen erfolgen beim jeweiligen Anbieter. Vorhandene Terminalausgaben werden nicht automatisch an einen Anbieter gesendet. Ein Neustart öffnet eine neue CLI-Sitzung; er setzt keinen KI-Chat automatisch fort.

## Bauen

Voraussetzungen: Windows x64, Rust Stable sowie entweder Microsoft C++ Build Tools mit Windows SDK (MSVC-Target) oder eine passende GNU-Linker-Toolchain.

```powershell
cargo test --locked
cargo build --release --locked
```

Alternativ: `./scripts/build.ps1` (kopiert auch das für Git Bash benötigte Startskript neben die EXE). Das ausführbare Programm liegt anschließend unter `target/release/rust-terminal.exe`.

PowerShell 7 muss im PATH, unter `C:\Program Files\PowerShell\7` oder portabel unter `shells/powershell/pwsh.exe` neben der EXE liegen. Die ältere Windows PowerShell 5.1 wird nicht als Ersatz verwendet. Unter **Einstellungen → Shells und KI-CLIs** können EXE- und CMD-Pfade explizit angegeben werden; relative Pfade gelten zum Programmordner.

## Portables Paket erstellen

```powershell
./scripts/package.ps1
```

Das Skript baut das Projekt, lädt die in `shells/powershell-source.json` festgeschriebene offizielle PowerShell-Version herunter, prüft SHA256 und erstellt `dist/TerminalVelocity-0.5.0-Windows-x64.zip`. Mit `-SkipBuild` lässt sich ein bereits gebautes Release verpacken. Bereits installierte KI-CLIs werden automatisch gesucht; sie werden nicht in das Paket kopiert.

Dieses Repository enthält die Quellen und die angepasste Terminalbibliothek. Kompilate, heruntergeladene Laufzeiten, Zugangsdaten, lokale Einstellungen und Testausgaben sind ausgeschlossen.

## Eigene Hintergrundbilder (0.5.0)

Unter **Einstellungen → Eigenes Hintergrundbild**:

1. **Datei auswählen …** öffnet den Windows-Dateidialog. Alternativ einen Pfad einfügen und **Laden / neu laden** wählen.
2. **Abdunklung** von 0 % (unverändert) bis 100 % (schwarz) einstellen; Standard ist 70 %.
3. **Ausfüllen** füllt den Terminalbereich mit mittigem Zuschnitt. **Einpassen** zeigt das ganze Bild mit freien Rändern. Das Seitenverhältnis bleibt erhalten.
4. **Hintergrundbild anzeigen** schaltet das Bild aus, ohne den Pfad zu vergessen. **Entfernen** entfernt nur die Zuordnung; die Bilddatei bleibt erhalten.

PNG, JPEG, WebP und BMP werden unterstützt. Das Bild liegt je Bereich hinter Text, Auswahl, Suche und den optionalen Grafikeffekten. Wer ausschließlich das Foto sehen möchte, stellt das Effekt-Preset auf **Aus**. Bild und Effekt-Preset sind unabhängig. Einstellungen werden im Workspace gespeichert; Projektprofile verwenden die globalen Darstellungseinstellungen.

Das Laden erfolgt über einen Hintergrundprozess mit maximal einem gleichzeitigen Auftrag. Die fertige GPU-Textur wird von allen Bereichen gemeinsam verwendet. Fehler werden in den Einstellungen angezeigt; beschädigte/fehlende Dateien werden nicht bei jedem Bildaufbau erneut geladen. **Laden / neu laden** liest eine inzwischen geänderte Datei erneut.

Grenzen: 32 MiB Dateigröße, 16 Megapixel, maximal 16384 Pixel pro Seite. Für die Anzeige werden große Bilder proportional auf maximal 2048 Pixel pro Seite reduziert. Animierte WebP-Dateien werden als Standbild behandelt. Gespeichert wird ausschließlich der Pfad; relative Pfade gelten zum Programmordner. Wird die Datei verschoben oder entfernt, muss der Pfad neu gewählt werden.

## Grafikeffekte (0.4.0)

Unter **Einstellungen → Grafikeffekte** stehen **Aurora**, **Phosphor** und **Deep Space** sowie **Aus** bereit. Aurora zeigt langsam bewegte türkis-violette Lichtbänder. Phosphor bietet dezente Scanlines und eine grüne oder bernsteinfarbene Standardschrift. Deep Space zeigt langsam wandernde Sterne. ANSI-Farben, Suchmarkierungen und Auswahl bleiben erhalten.

Der Regler **Effektstärke** steuert die dekorativen Hintergründe und das Cursor-Leuchten. **Animationen** schaltet Bewegung und Cursor-Schweif aus; die Hintergrundgestaltung bleibt statisch. **Cursor-Schweif** lässt sich zusätzlich separat abschalten. Aurora mit 45 % Intensität ist die Voreinstellung. Einstellungen werden mit dem Workspace gespeichert; Projektprofile behalten die globalen Darstellungseinstellungen.

Hintergründe werden hinter Terminalzellen gezeichnet, auf den jeweiligen Bereich begrenzt und nur im fokussierten aktiven Bereich animiert (20 angeforderte Hintergrundbilder pro Sekunde). Der Schweif umfasst maximal zwölf Spuren, klingt innerhalb von 0,32 Sekunden ab und wird bei Scrollen, Fokusverlust oder Layoutänderungen gelöscht. Inaktive Bereiche bleiben statisch. Das sind native egui/OpenGL-Effekte; Kitty-Shaderdateien lassen sich damit nicht importieren. Text-Nachleuchten ist noch nicht enthalten; eigene Bilder stehen seit 0.5.0 bereit.

## Remote-Verbindungen (0.3.0)

Über **Verbinden …** lassen sich SSH- und PowerShell/WinRM-Sitzungen in einem neuen Tab oder neben/unter dem aktiven Bereich öffnen. Ein Profil enthält Name, Host, optionales Konto und optionalen Port. Mit **Verbindung speichern** wird es lokal gespeichert; Favoriten stehen oben. Bestehende Profile lassen sich auswählen, ändern und erneut speichern oder nach Bestätigung löschen. **Verbinden** öffnet auch eine noch nicht gespeicherte Verbindung.

- **SSH:** OpenSSH `ssh.exe` wird direkt mit getrennten Argumenten gestartet. Ein leerer Port beziehungsweise ein leeres Konto lässt die vorhandene SSH-Konfiguration entscheiden. Hostaliase, SSH-Agent, `IdentityFile` und `ProxyJump` werden vom Client verarbeitet. Optional kann ein absoluter Schlüsselpfad angegeben werden. Hostschlüsselprüfung und Anmeldung erfolgen im Terminal. Ein eigener SSH-Programmpfad lässt sich in den Einstellungen setzen.
- **PowerShell / WinRM:** PowerShell 7 startet `Enter-PSSession`. Ohne Konto wird die aktuelle Windows-Identität verwendet; bei angegebenem Konto fragt PowerShell interaktiv nach dem Kennwort. HTTPS, Port und ein PowerShell-Konfigurationsendpunkt sind optional. Der Zielserver benötigt eine bereits eingerichtete Remoting-Konfiguration. Die Anwendung verändert keine Server-, Zertifikats- oder TrustedHosts-Einstellungen.
- Bereichsüberschriften zeigen Protokoll und Ziel. **Neu verbinden** startet eine frische Verbindung und leert die Ausgabe erst nach dem bewussten Neustart; solange der Prozess noch läuft, ist eine Bestätigung nötig. Nach Verbindungsende bleibt die Ausgabe lesbar. Der WinRM-Host beendet sich beim Rückfall in seinen lokalen Prompt, damit unter einer Remote-Überschrift keine lokale Shell weiterläuft.
- Remote-Ausgaben ändern niemals den lokalen Startordner. Neue lokale Tabs und KI-Übergaben verwenden diesen lokalen Ordner; die KI läuft auf diesem Computer. Entfernte Arbeitsordner werden in 0.3.0 nicht nachgeführt.
- Verbindungsprofile liegen in `workspace.json`. Projektprofile und gespeicherte Bereiche enthalten jeweils eine Kopie der Verbindung. Änderungen oder Löschen eines Favoriten verändern bestehende Bereiche/Projektprofile nicht. Beim Wiederherstellen des Workspace sowie beim Öffnen eines Projektprofils werden enthaltene Remote-Verbindungen neu aufgebaut; Prozesse oder Remote-Sitzungen werden nicht fortgesetzt.

Kennwörter und Schlüssel-Inhalte werden nicht gespeichert. Maximal 128 Verbindungsprofile sind möglich; die bisherigen Grenzen für Tabs und Bereiche gelten weiterhin. Alte Workspace- und Projektdateien bleiben lesbar. PowerShell über SSH, Dateitransfer und eine eigene Tunnelverwaltung gehören noch nicht zu dieser Version.

Für lokale Prüfungen ohne Server:

```powershell
cargo test --locked
# Zusätzlich: OpenSSH, PowerShell-Parser und WinRM-Hostende über ConPTY, ohne Netzwerkzugriff:
cargo test --locked -- --include-ignored
```

Die drei zusätzlichen Integrationstests benötigen lokal installiertes OpenSSH beziehungsweise PowerShell 7. Deshalb laufen sie nur auf ausdrücklichen Aufruf. Ein erfolgreicher Offline-Test bestätigt keine Anmeldung an einem echten SSH-/WinRM-Server.

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
| Verlauf durchsuchen | Strg+F |

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

## Suche im Terminalverlauf

Den gewünschten Bereich aktivieren und **Strg+F** oder **Suchen** wählen. Die Suche beachtet Groß- und Kleinschreibung und behandelt Sonderzeichen als normalen Text. Sie durchsucht den noch gespeicherten Verlauf dieses Bereichs, einschließlich weich umgebrochener Zeilen. Treffer erscheinen gelb, der aktuelle Treffer orange.

**Enter** / **Weiter** springt zum nächsten Treffer, **Umschalt+Enter** / **Zurück** zum vorherigen. Nach dem letzten Treffer geht es wieder zum ersten. Ältere Treffer werden in den sichtbaren Bereich gescrollt. **Escape** oder **Schließen (Esc)** entfernt die Hervorhebung und gibt den Fokus ans Terminal zurück; die erreichte Scrollposition bleibt erhalten. Danach lässt sich Text wie gewohnt markieren und über das KI-Kontextmenü untersuchen.

Neue Ausgabe wird während der Suche regelmäßig berücksichtigt. Beim Wechsel des Bereichs oder Öffnen eines Dialogs wird die Suche geschlossen. Suchtexte werden nicht gespeichert oder an die Shell bzw. KI gesendet. Maximal 256 Zeichen Suchtext und die ersten 10.000 Treffer werden unterstützt; ein `+` im Zähler weist auf weitere Treffer hin. Bereits aus dem begrenzten Terminalverlauf entfernte Ausgaben können nicht gefunden werden.

## Prüfung

GitHub Actions führt bei Pushes auf `main`, bei Pull Requests und auf manuellen Aufruf Windows-Prüfungen aus. Der Workflow verwendet Windows Server 2022 und Rust 1.89.0: Formatierung, sämtliche Haupttests einschließlich Offline-Remote-Prüfungen, Release-Build sowie die PTY-Prüfungen für Ordnerübernahme, Neustart, Profile und Suche. Es werden keine Remote-Server kontaktiert oder KI-Anfragen gestellt. Downloads von Rust und Cargo-Abhängigkeiten benötigen Internetzugriff. Derselbe Ablauf lässt sich lokal mit PowerShell 7 über `./scripts/ci.ps1` ausführen; OpenSSH und die Build-Voraussetzungen müssen installiert sein. Dies ersetzt keine grafischen Bedienungs- oder Treibertests.

`cargo test --locked` prüft unter anderem Workspace-Kompatibilität, Ordnerübernahme bei Fokuswechsel, Teilungen, Anbieterpfade und den nativen Codex-Start. Die gebaute EXE bietet weitere lokale Tests:

```powershell
./dist/RustTerminal/RustTerminal.exe --smoke-test
./dist/RustTerminal/RustTerminal.exe --directory-test
./dist/RustTerminal/RustTerminal.exe --restart-test
./dist/RustTerminal/RustTerminal.exe --profile-test
./dist/RustTerminal/RustTerminal.exe --search-test
```

Dabei werden Berichte neben der EXE erzeugt. Die Verzeichnis- und Neustarttests verwenden echte Windows-Pseudoterminals. Details und bekannte Grenzen stehen in [VERIFICATION.md](VERIFICATION.md).

Für den Offline-Übergabetest zunächst `cargo build --release --locked --features test-tools` ausführen. Die erzeugte `prompt-probe.exe` neben die Terminal-EXE kopieren und diese mit `--handoff-test` starten. Der Test verwendet echte PTYs und ein lokales Konsolen-Testprogramm; es werden keine KI-Anfragen gesendet.

## Technik und Lizenzen

Rust, eframe/egui 0.31, alacritty_terminal 0.25 und die angepasste egui_term 0.1.0. Alle Rust-Abhängigkeiten sind über `Cargo.lock` festgeschrieben.

Projektlizenz: [MIT](LICENSE). Änderungen an der eingebetteten Terminalbibliothek sind in [VENDOR_CHANGES.md](VENDOR_CHANGES.md) dokumentiert; deren ursprüngliche Lizenz bleibt erhalten. Weitere Abhängigkeitslizenzen sind in `THIRD_PARTY_LICENSES.txt` enthalten. PowerShell behält beim Verpacken seine mitgelieferten Lizenzdateien.



## Release nachvollziehen und erneut paketieren

`scripts/package.ps1` setzt PowerShell 7 voraus. Standardmäßig werden Tests und Release-Build ausgeführt. Mit `-OutputDirectory <neuer Ordner>` lässt sich ein frisches Ausgabeziel angeben; bestehende Paketordner werden nicht überschrieben. `CARGO_TARGET_DIR` wählt optional das Build-Verzeichnis. `-SkipBuild` darf nur verwendet werden, wenn die dortige EXE zum aktuellen Quellstand gehört.

Das Paket enthält `BUILD-INFO.json` mit Git-Commit, Änderungsstatus, rustc-/cargo-Versionen, Cargo.lock-Prüfsumme und SHA256 je Datei. Neben dem ZIP liegt dessen `.sha256`-Datei. Der PowerShell-Download wird nur bei passender festgeschriebener SHA256 verwendet; geprüfte Downloads werden wiederverwendet.

ZIP-Einträge sind sortiert und haben feste Zeitstempel. Zwei Packläufe mit identischen Quelldateien, Binärdateien und derselben .NET-Kompressionsumgebung ergeben dasselbe ZIP. Das ist keine Zusage bitidentischer EXE-Neukompilierung auf beliebigen Rechnern: dafür müssen auch Compiler, Windows SDK, Linker und Buildpfade übereinstimmen. Der hier geprüfte Compiler ist rustc 1.89.0, Target x86_64-pc-windows-msvc.

Versionsänderungen stehen in [CHANGELOG.md](CHANGELOG.md).
