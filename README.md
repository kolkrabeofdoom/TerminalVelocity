# TerminalVelocity

TerminalVelocity ist ein Windows-Terminal mit Tabs, geteilten Ansichten, SSH- und WinRM-Verbindungen, einem lokalen Texteditor und Bildanzeige. Vorhandene KI-Kommandozeilenprogramme lassen sich als eigene Sitzungen starten; markierte Terminalausgaben können nach Prüfung an sie übergeben werden.

**Version 0.9.2 · Windows x64 · MIT-Lizenz**

[Download und Release Notes](https://github.com/kolkrabeofdoom/TerminalVelocity/releases/latest) · [Änderungsverlauf](CHANGELOG.md) · [Prüfbericht](VERIFICATION.md)

## Inhalt

- [Installation und erster Start](#installation-und-erster-start)
- [Tabs, Bereiche und Arbeitsordner](#tabs-bereiche-und-arbeitsordner)
- [Terminalbedienung und Suche](#terminalbedienung-und-suche)
- [Shells und KI-Programme](#shells-und-ki-programme)
- [Text an eine KI übergeben](#text-an-eine-ki-übergeben)
- [SSH und PowerShell-Remoting](#ssh-und-powershell-remoting)
- [Projektprofile und Workspace](#projektprofile-und-workspace)
- [Lokaler Texteditor](#lokaler-texteditor)
- [Bilder im Terminal](#bilder-im-terminal)
- [Darstellung und Grafikeffekte](#darstellung-und-grafikeffekte)
- [Tastenkürzel](#tastenkürzel)
- [Gespeicherte Daten](#gespeicherte-daten)
- [Bekannte Grenzen](#bekannte-grenzen)
- [Aus den Quellen bauen](#aus-den-quellen-bauen)
- [Tests und Paketierung](#tests-und-paketierung)
- [Technik und Lizenzen](#technik-und-lizenzen)

## Installation und erster Start

1. Das Windows-x64-ZIP auf der [Release-Seite](https://github.com/kolkrabeofdoom/TerminalVelocity/releases/latest) herunterladen.
2. Den gesamten Inhalt in einen beschreibbaren Ordner entpacken.
3. `TerminalVelocity.exe` im entpackten Ordner starten.
4. Oben eine Shell auswählen und mit **+ Tab** eine Sitzung öffnen. **Einstellungen** zeigt erkannte Programme und erlaubt eigene Programmpfade.

Das Paket enthält PowerShell **7.6.7**, die portable Microsoft-ConPTY-Laufzeit **1.25.260930003**, das Git-Bash-Startskript und eine Sixel-Demo. `conpty.dll` und `OpenConsole.exe` müssen neben der Terminal-EXE bleiben. Es ist kein Installer erforderlich; Windows-Systemdateien werden nicht ersetzt.

CMD stammt aus Windows. Git Bash, OpenSSH und KI-CLIs müssen bei Bedarf separat installiert sein. Zugangsdaten und persönliche Konfigurationen sind nicht enthalten. Windows PowerShell 5.1 wird nicht als Ersatz für PowerShell 7 verwendet.

TerminalVelocity benötigt Windows x64 mit Pseudoterminal-Unterstützung und einen Grafiktreiber für den OpenGL-Renderer. Geprüft werden der lokale Windows-Build und ein Windows-Server-2022-CI-Ablauf; eine vollständige Kompatibilitätsmatrix für Windows-Versionen und Grafiktreiber liegt nicht vor.

### Aktualisieren

Die neue Version in einen eigenen Ordner entpacken. Für die bisherigen Einstellungen `workspace.json` und gegebenenfalls `profiles.json` aus dem alten Programmordner übernehmen. Relative Programm- und Hintergrundbildpfade beziehen sich auf den neuen Programmordner und müssen dort weiterhin stimmen. Die Anwendung vor dem Kopieren der Einstellungen schließen.

## Tabs, Bereiche und Arbeitsordner

**+ Tab** startet das oben ausgewählte Programm in einem neuen Tab. **Nebeneinander** und **Untereinander** teilen den aktiven Bereich. Teilungen können verschachtelt und ihre Trennlinien mit der Maus verschoben werden. Ein Klick aktiviert einen Bereich; seine Umrandung kennzeichnet den Fokus.

Neue lokale Tabs und Teilungen übernehmen den zuletzt gemeldeten Arbeitsordner des aktiven Bereichs. PowerShell, CMD und Git Bash melden Verzeichniswechsel am Prompt. Die Bereichsüberschrift zeigt diesen Ordner. Für Sitzungen ohne Shell-Prompt, etwa direkt gestartete KI-CLIs, bleibt der geerbte Startordner maßgeblich. Unter **Einstellungen → Startordner ohne aktiven Bereich** lässt sich der Rückfallordner festlegen.

**Sitzung neu starten** beendet den bisherigen Prozess und startet dasselbe Programm im übernommenen Ordner neu. Die Ausgabe dieses Bereichs wird geleert; andere Bereiche laufen weiter. Bei einer noch laufenden Sitzung wird vorher nachgefragt. Ein fehlender Startordner führt zu einer Fehlermeldung.

Mit **×** im Bereichskopf wird der Bereich geschlossen. Dabei endet auch seine Sitzung. Die Anwendung unterstützt insgesamt bis zu **32 Tabs und 64 Bereiche**.

## Terminalbedienung und Suche

Das Terminal verarbeitet Text, ANSI-Farben und Steuersequenzen. Konsolenprogramme können den alternativen Bildschirm und Mausberichte verwenden. Der Textverlauf umfasst bis zu **10.000 zusätzliche Zeilen je Bereich**.

- Mit der Maus ziehen, um Text zu markieren; Doppelklick wählt ein Wort beziehungsweise einen zusammengehörigen Textabschnitt, Dreifachklick eine Zeile.
- In Programmen mit eigener Maussteuerung beim Markieren **Umschalt** gedrückt halten.
- **Strg+Umschalt+C** kopiert die Auswahl, **Strg+Umschalt+V** fügt Text ein. Kopieren ist auch im Kontextmenü verfügbar.
- Das Mausrad scrollt im Verlauf. Erkannte Textlinks können mit **Strg+Klick** geöffnet werden.

### Verlauf durchsuchen

**Strg+F** oder **Suchen** durchsucht den Verlauf des aktiven Bereichs. Die Suche ist wörtlich und beachtet Groß- und Kleinschreibung; reguläre Ausdrücke werden nicht ausgewertet. Auch weich umgebrochene Textzeilen werden berücksichtigt.

Treffer erscheinen gelb, der aktuelle Treffer orange. **Enter / Weiter** und **Umschalt+Enter / Zurück** wechseln zwischen den Treffern; nach dem letzten folgt wieder der erste. Ältere Treffer werden in die Ansicht gescrollt. **Escape** schließt die Suche und erhält die erreichte Scrollposition.

Neue Ausgabe wird während der Suche berücksichtigt. Beim Bereichswechsel oder Öffnen eines Dialogs wird die Suche geschlossen. Suchtexte werden nicht gespeichert oder an die Shell gesendet. Grenzen: **256 Zeichen Suchtext**, **10.000 Treffer**; ein `+` im Zähler zeigt weitere Treffer an. Bereits aus dem Verlauf entfernte Zeilen und Bildinhalte sind nicht durchsuchbar.

## Shells und KI-Programme

| Eintrag | Verwendung und Voraussetzung |
| --- | --- |
| PowerShell 7 | Lokale Shell; im Download-Paket enthalten. |
| CMD | Windows-Eingabeaufforderung. |
| Git Bash | Bash aus einer vorhandenen Git-for-Windows-Installation. |
| Codex CLI | Separat installierte und eingerichtete CLI. |
| Claude Code | Separat installierte und eingerichtete CLI. |
| Gemini CLI | Separat installierte und eingerichtete CLI. |
| OpenAI API CLI | Vorhandenes `openai.exe`; für die Textübergabe sind Modell-ID und eingerichteter API-Zugang erforderlich. |

Unter **Einstellungen → Shells und KI-CLIs** werden gefundene Programme angezeigt. Leere Felder verwenden die automatische Suche; alternativ vollständige EXE-/CMD-Pfade oder Pfade relativ zum Programmordner eintragen. Für OpenSSH gibt es ein eigenes Feld. **Speichern und fehlgeschlagene Sitzungen neu starten** übernimmt die Einstellungen und versucht zuvor fehlgeschlagene Starts erneut. Laufende Sitzungen bleiben aktiv; geänderte Pfade gelten für neue Starts.

TerminalVelocity startet die jeweiligen Programme. Anmeldung, Kosten, Werkzeugberechtigungen und Bestätigungen werden von ihnen geregelt. Ein Sitzungsneustart stellt keinen vorhandenen KI-Chat wieder her.

## Text an eine KI übergeben

1. Terminaltext markieren.
2. Im Kontextmenü **An KI übergeben …** wählen.
3. Im Entwurf Anbieter, lokalen Projektordner und Text prüfen oder bearbeiten.
4. Mit **Senden und KI-Tab öffnen** eine neue Sitzung starten.

Die Schnellaktionen **Fehler erklären**, **Lösung vorschlagen**, **Befehl erklären** und **Ausgabe zusammenfassen** ergänzen eine deutsche Anweisung. Auch diese Entwürfe werden erst nach dem Senden übergeben. Ohne Textauswahl sind die Aktionen deaktiviert.

Auswahl und Zeilenumbrüche bleiben erhalten. Der vollständige Entwurf ist auf **10.000 UTF-16-Zeicheneinheiten** begrenzt. Die Übergabe erfolgt als Programmargument, nicht als auszuführender Shell-Befehl. **Abbrechen** verwirft den Dialog; Entwürfe werden nicht im Workspace gespeichert.

Die Schnellaktionen bitten um Erklärungen oder Vorschläge ohne Befehlsausführung. Das ist eine Anweisung an die CLI, keine technische Beschränkung ihrer Werkzeuge. Erst die ausdrücklich bestätigte Übergabe sendet die Auswahl an die gestartete Anbieter-CLI. Auch bei markierter Remote-Ausgabe läuft diese KI-Sitzung lokal.

## SSH und PowerShell-Remoting

**Verbinden …** öffnet den Verbindungsdialog. Verbindungen können in einem neuen Tab, neben oder unter dem aktiven Bereich starten. Einträge enthalten Name, Host, optionales Konto und optionalen Port. **Verbindung speichern** legt ein Profil an; Favoriten erscheinen zuerst. Profile lassen sich bearbeiten und nach Bestätigung löschen. **Verbinden** funktioniert auch ohne vorheriges Speichern.

### SSH

TerminalVelocity startet `ssh.exe` mit getrennten Argumenten. Bei leerem Konto oder Port gelten die Einstellungen des OpenSSH-Clients. Hostaliase, SSH-Agent, `IdentityFile` und `ProxyJump` werden durch dessen Konfiguration verarbeitet. Zusätzlich lässt sich im Dialog ein absoluter Schlüsseldateipfad angeben.

Hostschlüsselprüfung und Anmeldung finden im Terminal statt. Die Anwendung speichert weder Kennwörter noch private Schlüssel. Der SSH-Client muss installiert oder über einen eigenen Programmpfad erreichbar sein.

### PowerShell über WinRM

PowerShell 7 startet `Enter-PSSession`. Ohne Kontoeintrag wird die aktuelle Windows-Identität verwendet; andernfalls fragt PowerShell interaktiv nach dem Kennwort. HTTPS, Port und ein PowerShell-Konfigurationsendpunkt sind optional.

Der Server muss bereits für Remoting eingerichtet sein. TerminalVelocity ändert keine Serverkonfiguration, Zertifikate oder TrustedHosts-Einträge. PowerShell-Remoting über SSH ist derzeit nicht implementiert.

### Verbindungsende und Wiederherstellung

**Neu verbinden** startet eine neue Verbindung. Solange die alte Sitzung läuft, ist eine Bestätigung erforderlich. Nach Verbindungsende bleibt die Ausgabe sichtbar. Der WinRM-Host beendet sich beim Rückfall in den lokalen Prompt, damit unter der Remote-Überschrift keine lokale Shell weiterläuft.

Remote-Ausgaben ändern den lokalen Startordner nicht. Entfernte Arbeitsordner werden nicht nachgeführt. Verbindungsprofile werden im Workspace gespeichert; maximal **128 Profile** sind möglich. Offene Bereiche und Projektprofile besitzen eigene Kopien. Änderungen an einem Favoriten ändern diese Kopien nicht.

Beim Wiederherstellen des Workspace oder Öffnen eines Projektprofils werden darin enthaltene Remote-Verbindungen neu aufgebaut. Bestehende Prozesse oder Remote-Sitzungen werden nicht fortgesetzt. Dateitransfer und eine eigene Tunnelverwaltung sind nicht enthalten.

## Projektprofile und Workspace

Der **Workspace** beschreibt den zuletzt verwendeten Aufbau. **Workspace speichern** sichert ihn ausdrücklich; beim normalen Beenden wird er ebenfalls gespeichert. Beim nächsten Start werden Tabs und Bereiche mit neuen Prozessen aufgebaut. Ausgabe, Prozesse und KI-Chats werden nicht wiederhergestellt.

Ein **Projektprofil** ist eine benannte Vorlage für Tabs, Teilungen, Programme, Verbindungen und Bereichsordner:

1. Den gewünschten Aufbau herstellen und **Projekt speichern …** wählen.
2. Namen und vollständigen Projektordner eintragen, dann **Profil speichern**.
3. Später über **Projekt öffnen** das Profil auswählen und die Vorschau prüfen.
4. Mit **Projekt in neuen Tabs öffnen** neue Sitzungen starten. Bereits offene Tabs bleiben erhalten.

Bei einem geänderten Projektordner ziehen Bereiche innerhalb des ursprünglichen Ordners einschließlich Unterordnern mit um. Externe Ordner bleiben unverändert. Fehlende Ordner oder Programme werden vor dem Start gemeldet. Darstellung und Programmpfade bleiben globale Einstellungen.

Bis zu **32 Projektprofile** mit Namen von **1 bis 80 Zeichen** sind möglich. **Profil löschen …** entfernt eine Vorlage nach Bestätigung. Zum Ersetzen eines Profils einen neuen Namen verwenden oder die alte Vorlage zuerst löschen. Fehlerhafte Profildateien werden gemeldet und nicht automatisch überschrieben.

## Lokaler Texteditor

**Editor** öffnet einen eigenen Tab für ein Dokument. Die Terminalprozesse laufen währenddessen weiter. **Neu**, **Öffnen …**, **Speichern**, **Speichern unter …** und **Editor schließen** stehen in der Editorleiste bereit. Der Dateidialog startet im lokalen Ordner des aktiven Terminalbereichs.

Der Editor bietet Zeilennummern, horizontales und vertikales Scrollen, Einrückung mit Tab, Undo/Redo und eine wörtliche, groß-/kleinschreibungssensitive Suche. **Nächster Treffer** markiert den gefundenen Text. Einfache Syntaxfarben werden anhand der Endung für PowerShell, Bash, JSON, TOML und Markdown gewählt.

Ein Stern am Dokumentnamen zeigt ungespeicherte Änderungen. Beim Ersetzen, Schließen oder normalen Beenden kann gespeichert, verworfen oder abgebrochen werden. Der Wechsel zu einem Terminaltab erhält den Entwurf im Arbeitsspeicher. **Workspace speichern** speichert nicht den Editorinhalt.

Unterstützt werden **UTF-8 mit oder ohne BOM**, einheitliche **LF- oder CRLF-Zeilenenden** und Dateien bis **256 KiB**. Neue Dateien verwenden UTF-8 ohne BOM und LF. Andere Kodierungen, Binärdateien und gemischte Zeilenenden werden abgewiesen. Bestehende Kodierungs- und Zeilenendeneigenschaften bleiben beim Speichern erhalten.

Speichern erfolgt über eine temporäre Datei und atomaren Austausch. Externe Inhaltsänderungen, gelöschte Quelldateien oder Schreibschutz stoppen den Vorgang. Ein eigener Entwurf kann dann unter anderem Namen gespeichert werden. Es gibt keine automatische Wiederherstellung nach einem erzwungenen Prozessabbruch, keinen Sprachserver, keine Markdown-Vorschau und keine Remote-Dateibearbeitung. Der Editor führt keine Skripte aus.

## Bilder im Terminal

Unter **Einstellungen → Bilder im Terminal anzeigen (Sixel und lokale Dateien)** lässt sich die Bildanzeige gemeinsam ein- oder ausschalten. Abschalten entfernt die gespeicherten Terminalbilder. Ein Bild ist Terminalinhalt und vom dekorativen Hintergrundbild unabhängig.

### Lokale Bilder

Drei Wege stehen zur Verfügung:

- **Rechtsklick auf einen ausgegebenen Bilddateinamen → Bild öffnen** zeigt den erkannten lokalen Pfad direkt an. Der Tooltip nennt den ermittelten Pfad.
- **Rechtsklick → Bild auswählen …** öffnet den Windows-Dateidialog.
- Eine einzelne Bilddatei aus dem Explorer auf die Ausgabefläche eines Bereichs ziehen. Die Ablageposition bestimmt den Zielbereich.

Unterstützt werden **PNG, JPEG, WebP und BMP**. Die Datei wird im Hintergrund geladen und direkt als RGBA angezeigt; ein Sixel-Konverter ist dafür nicht nötig. Große Bilder werden proportional verkleinert, kleine nicht vergrößert. Transparenz bleibt erhalten. Animierte Dateien werden nicht als Animation wiedergegeben.

Das Bild erscheint an der aktuellen Ausgabeposition; eine begonnene Textzeile wird vorher abgeschlossen. Bilder scrollen mit dem Verlauf. Laufende Programme können Bildbereiche später überschreiben oder löschen. Bei hohen Bildern ist der obere Teil über den Scrollback erreichbar.

Relative angeklickte Pfade gelten zum aktuellen lokalen Sitzungsordner. Terminaltext enthält keine verlässlichen Dateimetadaten: abgeschnittene Namen, mehrdeutige Auflistungen und ältere Ausgaben aus anderen Ordnern können falsch oder gar nicht zugeordnet werden. Hier hilft die separate Dateiauswahl. In SSH-/WinRM-Bereichen ist direktes Öffnen von Textpfaden deaktiviert; lokale Auswahl und Ablage bleiben möglich. Es erfolgt kein Upload an die Gegenstelle.

Es wird jeweils eine Datei geladen. Wird die Zielsitzung geschlossen oder neu gestartet oder ihre Breite während des Ladens geändert, wird das Ergebnis verworfen. Fehler stehen in der Statusleiste. Im Editor und bei geöffneten App-Dialogen ist die Ablage deaktiviert.

### Sixel-Ausgabe aus Programmen

Ein Programm kann Bilder als Sixel-Escape-Sequenzen ausgeben, auch über eine SSH-Sitzung. TerminalVelocity decodiert diese Daten und zeichnet das Bild zwischen den Textzeilen. Zum lokalen Ausprobieren im PowerShell-Tab aus dem entpackten Programmordner:

```powershell
& .\shells\sixel-demo.ps1
```

Die Demo erzeugt ein 240 × 120 Pixel großes Farbmuster und schreibt danach darunter weiter. Sie benötigt keine zusätzlichen Programme und kein Netzwerk.

Unterstützt werden 256 Farbregister, RGB/HLS-Farbangaben, Wiederholungen, Rasterabmessungen, Pixelseitenverhältnis, transparente beziehungsweise gefüllte Hintergründe, Grafikfähigkeits- und Zellgrößenabfragen sowie DEC-Modi 80 und 1070. Haupt- und Alternativbildschirm besitzen getrennte Bildbestände. Textüberschreiben und Löschbefehle entfernen betroffene Segmente.

Die mitgelieferte ConPTY-Laufzeit ist für den Transport wichtig: Ältere Windows-Systemversionen können Sixel-Sequenzen entfernen. Die Implementierung ist keine vollständige VT340-Emulation und unterstützt nicht das Kitty-Grafikprotokoll.

### Bildgrenzen

| Gegenstand | Grenze oder Verhalten |
| --- | --- |
| Lokale Quelldatei | 32 MiB, 16 Megapixel, höchstens 16384 Pixel je Seite |
| Lokales Bild nach Decodierung | Zunächst höchstens 2048 Pixel je Seite, danach Verkleinerung auf Terminalbreite und Bildlimit |
| Einzelnes gespeichertes Terminalbild | Höchstens 4096 Pixel je Seite und 2 Megapixel |
| Einzelne Sixel-Sequenz | Höchstens 4 MiB Eingabedaten |
| Bildbestand je Bildschirm | 16 MiB unkomprimierte Bilddaten, 32 Bilder und 4096 Zeilensegmente; ältere Bilder werden bei Bedarf entfernt |
| Fenster-, Layout- oder Schriftgrößenänderung | Vorhandene Terminalbilder werden gelöscht |
| Zeichen einfügen/löschen | Betroffene Bildsegmente ab der Position werden entfernt |

Terminalbilder werden nicht kopiert, durchsucht oder im Workspace gespeichert. Zoom, Bildexport und ein eigener Bildanzeigebefehl sind noch nicht vorhanden.

## Darstellung und Grafikeffekte

### Schrift und Transparenz

Die Schriftgröße lässt sich zwischen **10 und 28** einstellen. Wenn vorhanden, wird Consolas geladen; ansonsten greift die Schriftkonfiguration des Renderers.

**Kompatible Fenstertransparenz** verwendet die Windows-Fensterdeckkraft. Dabei werden auch Text und Bedienelemente transparent. Bei abgeschaltetem Kompatibilitätsmodus betrifft die Transparenz die Hintergrundflächen und hängt von der Grafiktreiberunterstützung ab. **Hintergrund weichzeichnen (Windows-Acryl)** ist für diesen Modus optional. Standardmäßig ist der Kompatibilitätsmodus aktiv.

### Effekt-Presets

| Preset | Darstellung |
| --- | --- |
| Aus | Keine dekorativen Grafikeffekte |
| Aurora | Türkis-violette Lichtbänder |
| Phosphor | Scanlines und grüne oder bernsteinfarbene Standardschrift |
| Deep Space | Sternpartikel |

**Effektstärke** steuert die Hintergründe und das Cursor-Leuchten. **Animationen** schaltet Bewegung ab; die Hintergrundgestaltung bleibt statisch. **Cursor-Schweif** lässt sich gesondert deaktivieren. Animationen laufen nur im aktiven Bereich bei fokussiertem Fenster. ANSI-Farben, Auswahl und Suchmarkierungen bleiben erhalten. Voreinstellung ist Aurora mit 45 % Stärke.

Im Phosphor-Preset kann **Text-Nachleuchten** neue und veränderte Zeichen kurz hervorheben. **Leuchtstärke** und **Nachleuchtdauer** sind einstellbar; Standard sind 50 % und 0,6 Sekunden, der Dauerbereich liegt bei 0,1 bis 1,5 Sekunden. Gelöschte Zeichen hinterlassen keine Textreste. Bei Auswahl, Suchtreffern, Fokusverlust oder deaktivierten Animationen wird der Effekt unterdrückt. Scroll- und Layoutänderungen setzen ihn zurück.

Das sind eingebaute egui/OpenGL-Effekte. Kitty-Shaderdateien können nicht importiert werden.

### Eigene Hintergrundbilder

Unter **Einstellungen → Eigenes Hintergrundbild** eine Datei auswählen oder einen Pfad eingeben und **Laden / neu laden** wählen. **Abdunklung** reicht von 0 % bis 100 %, Standard sind 70 %.

**Ausfüllen** nutzt einen mittigen Zuschnitt, **Einpassen** zeigt das ganze Bild mit freien Rändern. Das Seitenverhältnis bleibt erhalten. **Hintergrundbild anzeigen** schaltet die Darstellung aus, ohne den Pfad zu vergessen. **Entfernen** löscht nur die Zuordnung, nicht die Datei.

PNG, JPEG, WebP und BMP werden unterstützt. Es gelten dieselben Quelldateigrenzen wie für lokale Terminalbilder; die Hintergrundtextur wird auf höchstens 2048 Pixel je Seite reduziert. Das Laden erfolgt im Hintergrund, die fertige Textur wird zwischen Bereichen geteilt. **Laden / neu laden** liest eine inzwischen geänderte Datei erneut. Fehler werden in den Einstellungen angezeigt.

Hintergrundbild und Effekt-Preset sind unabhängig. Das Bild liegt hinter Terminalinhalt und Effekten. Gespeichert wird der Dateipfad, keine Bildkopie; relative Pfade gelten zum Programmordner. Nach dem Verschieben einer Bilddatei muss der Pfad angepasst werden.

## Tastenkürzel

### Terminal

| Aktion | Tastenkürzel |
| --- | --- |
| Neuer Tab | Strg+Umschalt+T |
| Bereiche nebeneinander | Strg+Umschalt+D |
| Bereiche untereinander | Strg+Umschalt+E |
| Nächster Bereich | Strg+Tab |
| Nächster Tab | Strg+Umschalt+Pfeil rechts |
| Bereich schließen | Strg+Umschalt+W |
| Workspace speichern | Strg+Umschalt+S |
| Kopieren / Einfügen | Strg+Umschalt+C / Strg+Umschalt+V |
| Verlauf durchsuchen | Strg+F |
| Nächster / vorheriger Suchtreffer | Enter / Umschalt+Enter |
| Suche schließen | Escape |
| Erkannten Textlink öffnen | Strg+Klick |

### Editor

| Aktion | Tastenkürzel |
| --- | --- |
| Speichern | Strg+S |
| Speichern unter | Strg+Umschalt+S |
| Suchen | Strg+F |
| Rückgängig / Wiederholen | Strg+Z / Strg+Umschalt+Z oder Strg+Y |
| Einrücken / Einrückung entfernen | Tab / Umschalt+Tab |

Die Kürzel gelten im jeweiligen Bedienkontext. Insbesondere öffnet Strg+Umschalt+S im Editor „Speichern unter“ und speichert dort nicht den Workspace.

## Gespeicherte Daten

| Datei oder Inhalt | Speicherung |
| --- | --- |
| `workspace.json` neben der EXE | Tabs, Teilungen, Programme, zuletzt gemeldete lokale Ordner, Verbindungsprofile und Darstellungseinstellungen |
| `profiles.json` neben der EXE | Benannte Projektvorlagen einschließlich ihrer Verbindungskopien |
| Hintergrundbilder | Nur der gewählte Dateipfad im Workspace |
| Terminalausgabe, Terminalbilder, Suchtext | Nicht im Workspace gespeichert |
| KI-Übergabeentwürfe, Editorentwurf und Undo-Verlauf | Nur im Arbeitsspeicher |
| SSH-/WinRM-Kennwörter, private Schlüssel | Werden von TerminalVelocity nicht gespeichert |

Die JSON-Dateien sind lokale Konfigurationsdateien und können Hostnamen, Kontonamen und Dateipfade enthalten. Die gestarteten Shells und KI-CLIs verwalten ihre eigenen Verläufe, Zugänge und Konfigurationen unabhängig davon.

## Bekannte Grenzen

Die Anwendung befindet sich in Entwicklung. Es gibt automatisierte Prüfungen, aber keine vollständige manuelle Abnahme aller Bedienwege und Grafiktreiber.

- Echte SSH-/WinRM-Anmeldungen wurden bisher nicht geprüft; die Verbindungstests laufen lokal ohne Serverzugriff.
- Native Windows-Dateidialoge, Explorer-Ablage und das direkte Bildöffnen per Rechtsklick sind noch nicht vollständig im sichtbaren Fenster geprüft. Eingabe- und Renderlogik werden automatisiert getestet.
- Terminalbilder überstehen Größenänderungen nicht. Sixel ist begrenzt implementiert; Kitty-Grafikprotokoll und benutzerdefinierte Shader fehlen.
- Der Editor bearbeitet ein kleines UTF-8-Dokument zur Zeit und bietet keine Wiederherstellung nach einem erzwungenen Abbruch.
- Workspace und Projektprofile starten neue Prozesse. Sie speichern keine laufenden Sitzungen oder KI-Chats.

Der genaue Prüfstand einschließlich früherer Versionen steht in [VERIFICATION.md](VERIFICATION.md).

## Aus den Quellen bauen

Voraussetzungen für den geprüften Build: **Windows x64**, **PowerShell 7**, **Rust 1.89.0** mit `rustfmt` sowie **Microsoft C++ Build Tools und Windows SDK** für `x86_64-pc-windows-msvc`. Neuere Toolchains können funktionieren; der dokumentierte CI-Ablauf verwendet 1.89.0.

Im Repositoryordner:

```powershell
./scripts/build.ps1
```

Das Skript bereitet die festgeschriebene ConPTY-Laufzeit vor, führt die regulären Tests aus, erstellt den Release-Build und kopiert das Git-Bash-Startskript sowie die Sixel-Demo. Ergebnis: `target/release/TerminalVelocity.exe` und die zugehörigen Laufzeitdateien. `CARGO_TARGET_DIR` kann das Build-Verzeichnis ändern. Beim ersten Lauf werden Cargo-Abhängigkeiten und ConPTY heruntergeladen.

Ein normaler Build verpackt PowerShell noch nicht. Für den Start muss PowerShell 7 installiert oder unter `shells/powershell/pwsh.exe` neben der EXE verfügbar sein. Das portable Paket ergänzt diese Laufzeit automatisch.

Bei direkten Cargo-Aufrufen vorher die Laufzeit vorbereiten:

```powershell
./scripts/prepare-conpty.ps1
cargo test --locked
cargo build --release --locked
```

Für einen vollständigen Programmordner einschließlich der Hilfsskripte ist `build.ps1` vorzuziehen.

## Tests und Paketierung

### Prüfungen

```powershell
./scripts/ci.ps1
```

Der Ablauf prüft Formatierung, die Haupttests einschließlich ausdrücklich aktivierter Offline-Integrationstests, die vte-Parser-Tests, den Release-Build und vier reale Windows-PTY-Prüfungen: Ordnerübernahme, Sitzungsneustart, Projektprofile und Verlaufssuche. PowerShell 7 und OpenSSH müssen lokal verfügbar sein. Die Tests stellen keine Remote-Verbindung her und senden keine KI-Anfragen; notwendige Downloads beim Vorbereiten des Builds benötigen Internet.

Der [Windows-Workflow](.github/workflows/windows.yml) führt denselben Ablauf bei Pushes auf `main`, bei Pull Requests und auf manuellen Aufruf aus. Die lokale Suite umfasst derzeit **112 Tests** plus die vier PTY-Prüfungen.

Weitere Diagnosemodi der EXE sind `--smoke-test`, `--directory-test`, `--restart-test`, `--profile-test` und `--search-test`. Sie erzeugen Berichte neben der EXE. Für `--handoff-test` zuvor mit `--features test-tools` bauen und die erzeugte `prompt-probe.exe` neben die Terminal-EXE legen. Dieser Test verwendet ein lokales Probeprogramm statt einer KI-Anfrage.

### Portables Paket

```powershell
./scripts/package.ps1
```

Das Skript führt Tests und Build aus, ergänzt die festgeschriebenen PowerShell- und ConPTY-Dateien und erstellt `dist/TerminalVelocity-0.9.2-Windows-x64.zip` sowie eine `.sha256`-Datei. Downloadquellen und Prüfsummen stehen in [powershell-source.json](shells/powershell-source.json) und [conpty-source.json](shells/conpty-source.json).

Mit `-OutputDirectory <neuer Ordner>` wird ein anderes Ziel verwendet; bestehende Paketordner werden nicht überschrieben. `-SkipBuild` ist nur für einen bereits zum aktuellen Quellstand passenden Release-Build gedacht. KI-CLIs, persönliche Einstellungen und Zugangsdaten werden nicht eingepackt.

`BUILD-INFO.json` im Paket dokumentiert Git-Commit, Änderungsstatus, Toolchain, Cargo.lock-Prüfsumme, Laufzeitversionen und SHA256 je Datei. ZIP-Einträge sind sortiert und haben feste Zeitstempel. Identische Eingabedateien ergeben mit derselben Kompressionsumgebung identische ZIPs; bitidentische Neukompilierung auf beliebigen Rechnern wird damit nicht zugesichert.

Die ZIP-Prüfsumme lässt sich mit PowerShell ermitteln und mit der veröffentlichten `.sha256`-Datei vergleichen:

```powershell
Get-FileHash .\TerminalVelocity-0.9.2-Windows-x64.zip -Algorithm SHA256
```

## Technik und Lizenzen

Die Anwendung ist in Rust geschrieben. Die Oberfläche verwendet eframe/egui 0.31 mit OpenGL; Terminalzustand und Parser basieren auf angepassten Versionen von egui_term 0.1.0, alacritty_terminal 0.25.1 und vte 0.15.0. Die Abhängigkeiten sind in `Cargo.lock` festgeschrieben. Lokale Bibliotheksänderungen sind in [VENDOR_CHANGES.md](VENDOR_CHANGES.md) beschrieben.

TerminalVelocity steht unter der [MIT-Lizenz](LICENSE). Abhängigkeiten behalten ihre jeweiligen Lizenzen; siehe [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt), die Lizenzdateien in `vendor/` und die mitgelieferten PowerShell-/ConPTY-Lizenzen. Das Repository enthält keine kompilierten Programme, heruntergeladenen Laufzeiten oder persönlichen Workspace-Dateien.
