# Prüfung

Stand: 16. Oktober 2026, Windows x64.

- Vierzehn Rust-Regressionstests bestehen: Workspace-Kompatibilität, Anbieter- und Pfadauflösung, Layout-Roundtrip, doppelte IDs, bereichsbezogene Ordnerübernahme, Prüfung der KI-Übergabeargumente, Schnellaktionen und Projektprofile.
- Release-Build mit GNU-Rust-Toolchain erfolgreich.
- PowerShell und CMD: Eingabe und Ausgabe über echte Pseudoterminals geprüft.
- Ordnerwechsel und Start eines neuen Bereichs im übernommenen Ordner geprüft, einschließlich Leerzeichen, Apostroph, Unicode sowie kurzen und langen Windows-Pfadnamen.
- Neustart einer laufenden Sitzung: frischer Prozesszustand und neue Ausgabe, aktueller Ordner erhalten.
- Neustart nach Prozessende und nach einem Startfehler geprüft.
- Verspätete Ereignisse alter Sitzungen werden verworfen; Nachbarbereich und Anbieter bleiben erhalten.
- Codex, Claude, Gemini und OpenAI CLI wurden in der lokalen Installation gestartet. Die Anbieterprogramme und deren Anmeldungen sind nicht Bestandteil des Repositorys.
- Windows-Kompatibilitätstransparenz wird über die Windows-API gesetzt und rückgelesen. Der Benutzer hat die sichtbare Funktion bestätigt.
- Der Benutzer hat die Ordnerübernahme im Terminal bestätigt.
- Version 0.1.8: Mehrzeilige Terminalauswahl über eine echte PowerShell-PTY geprüft. Alle vier Anbieterprofile wurden mit einem lokalen Konsolen-Testprogramm auf unveränderte Argumente und den richtigen Projektordner geprüft, einschließlich Umlauten, Zeilenumbrüchen, Anführungszeichen, Shell-Sonderzeichen, führenden Optionszeichen und Dateiverweiszeichen. Der direkte Gemini-/Node-Start wurde zusätzlich mit `--version` geprüft. Dabei wurden keine KI-Anfragen gesendet.
- Nach der Argumentmaskierung bestehen erneut die Shell-, Ordner- und Neustarttests mit echten PTYs.
- Version 0.1.9: Release-Build erfolgreich. Alle vier Schnellaktionen erhalten die mehrzeilige Auswahl unverändert und verwenden die vorhandene Größenprüfung. Der erweiterte Offline-Übergabetest prüft die vollständigen Schnellaktions-Prompts für alle vier Anbieterprofile über echte Windows-PTYs, einschließlich Sonderzeichen und Projektordner. Keine KI-Anfragen gesendet. Der grafische Klickpfad der neuen Menüeinträge wurde nicht automatisiert geprüft.
- Version 0.2.0: Release-Build erfolgreich. Profiltests prüfen Layout und Anbieter, neue IDs beim Anhängen, unveränderte globale Einstellungen, verschobene Projekt-/Unterordner und unveränderte externe Ordner. Fehlende Ordner, doppelte Namen und überschrittene Layoutgrenzen werden abgewiesen. Persistenz, atomarer Austausch, ungültige JSON-Dateien und Entfernen von Profilen wurden geprüft.
- Der neue `--profile-test` startet echte CMD-PTYs: ein Profil mit Teilung öffnet in zusätzlichen Tabs, die neuen Prozesse haben frischen Zustand und die richtigen Projekt-/Unterordner. Bereits laufende Sitzungen behalten ihre IDs und ihren Prozesszustand. Fehlende Ordner verändern weder Tabs noch Sitzungen. Der Neustarttest und der PowerShell-/CMD-Shelltest bestehen weiterhin. Keine KI-Anfragen gesendet.

- Version 0.2.1: Release-Build und die 14 Rust-Regressionstests erfolgreich. Der neue `--search-test` prüft mit einer echten CMD-PTY 140 wörtliche Treffer im Scrollback, Vor-/Rückwärtssuche und Umlauf, Scrollen zu älteren Treffern, Highlight-Zuordnung, weiche Zeilenumbrüche, Unicode, Groß-/Kleinschreibung, neue Ausgabe, das Eingabelimit und Leeren der Suche. Die vorhandenen Shell-, Profil- und KI-Übergabetests bestehen weiterhin. Keine KI-Anfragen gesendet.

Nicht praktisch geprüft: Git Bash (lokal nicht installiert), alle Grafikkarten und Windows-Versionen, umfassende Kompatibilität mit interaktiven Terminalprogrammen sowie die grafischen Klickpfade von Neustart, KI-Kontextmenü, Projektprofilen und Verlaufssuche einschließlich des sichtbaren Tastaturfokus. Die Suchlogik und Highlight-Zuordnung wurden am echten Terminalgrid geprüft; die grafische Darstellung noch nicht per Klicktest. Die Übergabe wurde offline mit einem Testprogramm geprüft; tatsächliche Modellantworten und Anbieteranmeldungen bleiben dem interaktiven Betrieb vorbehalten.

Das portable Paketskript prüft die SHA256-Summe der festgeschriebenen PowerShell-ZIP vor dem Entpacken. Anbieter-CLIs benötigen eigene Installation und Anmeldung. Die Arbeitsbereichsdatei speichert keine laufenden Prozesse, Terminalausgaben oder KI-Chat-Verläufe.



## Lokale Nachprüfung und Version 0.3.0 – 19. Oktober 2026

Windows x64, Rust Stable/MSVC. 17 reguläre Tests und drei explizit aktivierte Offline-Integrationstests bestehen (`cargo test --locked -- --include-ignored`, insgesamt 20).

Neu geprüft: sichere SSH-Argumenttrennung und Hostvalidierung; OpenSSH-Konfigurationsauswertung mit Hostalias, Konto, Port und ProxyJump über `ssh -G` ohne Verbindung; PowerShell-Parser mit Sonderzeichen in Konto/Endpunkt und ersetzten Netzwerk-/Anmelde-Cmdlets; Ende des WinRM-Hostprozesses nach simulierter Sitzungsrückkehr über eine echte Windows-PTY. Der letzte Test deckte einen zunächst verbleibenden lokalen Prompt auf; der finale lokale Prompt beendet nun explizit den Hostprozess. Ebenfalls geprüft: alte Pane-Daten ohne Remote-Feld, Roundtrip von Remote-Layouts, Projektprofil-Übernahme mit neuen IDs, unabhängige Profilkopien und Ausschluss von Remote-Titeln bei lokalen Ordneränderungen.

Git Bash: Start und Ordnerübernahme wurden inzwischen praktisch über PTYs geprüft (inklusive Leerzeichen, Apostroph und Unicode). Beim direkten Cargo-Build muss shells/terminalvelocity.bash neben der EXE liegen; scripts/build.ps1 liefert es nun ebenfalls mit.

Echte SSH-/WinRM-Anmeldungen wurden auf Wunsch ausschließlich durch Offline-Tests ersetzt; keine Serververbindung oder KI-Anfrage. Grafische Klickpfade der Remote-Dialoge wurden nicht automatisiert geprüft. Die Computer-Use-Anleitung untersagt Terminal-App-Automatisierung. Authentifizierung, erreichbare Server, Zertifikate und reale Remote-Abbrüche bleiben praktisch ungeprüft.

Der finale 0.3.0-Release-Build und alle sechs PTY-Modi (--smoke-test, --directory-test, --restart-test, --profile-test, --search-test, --handoff-test) bestehen. Der Shelltest prüft PowerShell, CMD, Git Bash, Codex und Gemini; Claude und OpenAI API CLI sind hier nicht installiert und werden übersprungen.


## Grafikeffekte 0.4.0 – 19. Oktober 2026

Release-Build erfolgreich. 23 Aufrufe der Haupt-Test-Suite erfolgreich (einschließlich drei Remote-Offline-Integrationstests und eines Renderdaten-Exports). Ein zusätzlicher Test der tatsächlich eingebetteten Rendererdatei prüft Begrenzung/Verblassen des Cursor-Schweifs sowie Zurücksetzen bei Scrollen, Fokuswechsel, Größenänderung und verschwundenem Cursor. Die Tests prüfen auch persistierte Darstellungseinstellungen, Intensitätsgrenzen, vollständiges Ausschalten, statische Ausgabe bei deaktivierten Animationen und gültige, begrenzte Rendergeometrie.

Alle sechs bestehenden PTY-Testmodi bestehen mit 0.4.0. Nach der abschließenden Farbanpassung wurden die drei Effektprüfungen und der Release-Build erneut erfolgreich ausgeführt. Die Hintergrundgeometrie wurde softwaregerendert und visuell kontrolliert; die Phosphor-Intensität wurde dabei reduziert. Die Vorschau enthält ergänzten Beispieltext und ist kein Screenshot der Anwendung. Grafische Bedienung, Bewegung auf einem echten Monitor und GPU-/Treiberverhalten wurden nicht praktisch geprüft.

Der separate Renderer-Test wurde über eine lokale Testhülle unter work/effects-tests gegen egui 0.31.1 ausgeführt, da die vorhandene alte Vendor-Lockdatei eine offline nicht verfügbare Abhängigkeit verlangt. Die eingebettete Rendererdatei selbst enthält den Test.


## Hintergrundbilder 0.5.0 – 19. Oktober 2026

27 Hauptprüfungen einschließlich expliziter Offline-Integrationstests und Renderexport bestanden. Neue Prüfungen: PNG/JPEG/WebP/BMP-Decodierung; PNG-Transparenz; ungültige/fehlende Dateien; proportionale Texturreduktion auf 2048 Pixel; Ablehnung übergroßer Bildabmessungen; alte Einstellungen ohne Bildfelder; ungültige Abdunklungswerte; ignorierte überholte Ladeergebnisse; keine automatische Wiederholung bei Ladefehlern; Rücksetzen beim Abschalten; horizontaler/vertikaler Zuschnitt und Einpassen; Begrenzung der Zeichenoperationen auf den jeweiligen Bereich.

Der native Dateidialog und die Darstellung eines echten Bildes im laufenden Terminal wurden nicht per UI-Automation geprüft. Reale SSH-/WinRM-Verbindungen bleiben wie vereinbart außerhalb der Offline-Prüfung.

Der finale Release-Build 0.5.0 und alle sechs PTY-Testmodi (Shells, Ordnerübernahme, Neustart, Profile, Suche und KI-Übergabe) sind erfolgreich abgeschlossen.

## Einheitlicher Projektname 0.5.1 – 19. Oktober 2026

Formatierung, alle 27 Hauptprüfungen einschließlich Offline-Remote-Tests, Release-Build und die vier PTY-Prüfungen (Ordner, Neustart, Profile, Suche) erfolgreich. Die umbenannte TerminalVelocity.exe besteht auch den Shelltest für PowerShell, CMD, Git Bash, Codex und Gemini; Claude und OpenAI API CLI sind lokal nicht installiert. Neue Prompt-Kennungen und der neue Git-Bash-Skriptpfad werden durch die Ordnerprüfungen erfasst. Keine echten Remote-Verbindungen oder KI-Anfragen.

## Text-Nachleuchten 0.6.0 – 19. Oktober 2026

28 Haupttests und drei Renderer-Tests bestanden. Neu geprüft: alte Einstellungsdateien, ungültige Stärke/Dauer, Speicherung, Änderungserkennung auch nach Leerlauf, Ablauf ohne Auffrischen bei stabilem Text, sofortiges Löschen, Grenzen für Glyphen und Lichtpunkte, Rücksetzen bei Layout/Schrift/Scrollback/Scrollposition/Alternativbildschirm sowie Clipping und Ende der Zeichenoperationen nach Ablauf. Release-Build und die vier PTY-Prüfungen (Ordner, Neustart, Profile, Suche) erfolgreich. Keine echten Remote-Verbindungen. Die Bewegung und Wirkung auf einem realen Monitor sind noch nicht visuell bestätigt.
## Lokaler Editor 0.7.0 – 20. Oktober 2026

37 Haupttests und drei Renderer-Tests bestehen. Neun Editorprüfungen decken UTF-8/BOM/LF/CRLF-Erhaltung, Unicode-Dateinamen, atomaren Austausch, neue Dateien und Speichern unter, externe Änderungen, gelöschte Quelldateien, Namenskollisionen, Schreibschutz, Größen-/Formatgrenzen, Syntaxlayout ohne Textveränderung sowie den Schutz ungespeicherter Dokumente ab. Ein egui-Test speist Unicode-Text und Tastaturereignisse ein und prüft Undo, Redo, Tab-Einrückung und die korrekte Unicode-Auswahl eines Suchtreffers. Ein Viewport-Test bestätigt CancelClose bei ungespeicherten Änderungen.

Formatierung, Release-Build und die vier PTY-Modi für Ordnerübernahme, Neustart, Profile und Suche erfolgreich. Native Windows-Dateidialoge und die vollständige Bedienung im laufenden Fenster wurden nicht manuell geprüft. Keine Remote-Verbindungen, Datei-Uploads oder KI-Anfragen.

## Sixel 0.8.0 – 20. Oktober 2026

Lokaler CI-Ablauf erfolgreich: Formatierung, 37 Haupttests einschließlich Offline-Remote-Prüfungen, drei Effekt-Renderer-Tests, zehn Sixel-Tests, zwei echte ConPTY-Tests, 54 Upstream-vte-Parserprüfungen, Release-Build und vier PTY-Modi (Ordner, Neustart, Profile, Suche).

Die Sixel-Tests prüfen RGB/HLS, Transparenz/Hintergrund, Wiederholungen, Rastergrößen, Seitenverhältnis, Farbebenen, jede Zweiteilung einer vollständigen Bildsequenz, Abbruch/unvollständige Daten, Wiederaufnahme normalen Texts, ungültige/überdimensionierte Eingaben, gezieltes Überschreiben/Löschen, Scrollback, Alternativbildschirm, Größenänderung, Abfragen/Modi, private/geteilte Paletten und Bildbegrenzung. Zusätzlich werden 500 deterministisch erzeugte fehlerhafte Bytefolgen verarbeitet.

Ein realer PowerShell-Kindprozess sendet Sixel durch die gebündelte ConPTY-Laufzeit. Der zweite Integrationstest führt diese Ausgabe durch das echte Backend und den egui-Renderer: korrekte rote RGBA-Pixel, 12 × 6-Textur, Zeichenprimitive mit derselben Textur-ID und Texturfreigabe beim Abschalten werden geprüft. Dies ist ein Test ohne sichtbares Fenster, kein Nachweis der Ausgabe auf einem realen Bildschirm. Die lokal vorhandene System-ConPTY-Version hatte Sixel im vorgelagerten Transportversuch entfernt; die gebündelte Version reicht die Sequenz durch.

Keine vollständige VT340-Konformitätsprüfung, kein Test aller externen Bildprogramme und keine reale SSH-/WinRM-Verbindung. GPU-/Treiberverhalten und grafische Bedienung bleiben manuell zu prüfen. Bibliotheks-Unit-Tests von alacritty_terminal wurden nicht separat ausgeführt; dessen geänderte Terminalpfade werden über die zehn Sixel-Integrationstests geprüft.

## Lokale Bildanzeige 0.9.0 – 21. Oktober 2026

Lokaler CI-Ablauf erfolgreich: 41 Haupttests, drei Effekt-Renderer-Tests, elf Bild-/Sixel-Tests, zwei ConPTY-Tests und 54 Parserprüfungen (insgesamt 111). Formatierung, Release-Build und vier PTY-Modi für Ordnerübernahme, Neustart, Profile und Suche ebenfalls erfolgreich.

Neu geprüft: proportionale Verkleinerung ohne Vergrößerung, Alpha-Erhaltung und Pixellimit; Ablage genau einer lokalen Datei; egui-Ablageereignis nur im Bereich unter dem Mauszeiger und nur einmal; asynchrones Laden mit Zielkennung, Begrenzung auf einen Ladeauftrag und Fehlerrückgabe; direkte RGBA-Anzeige ohne Farbpalettenkonvertierung, Erhaltung einer begonnenen Textzeile, unterschiedliche Bild-IDs, gemeinsames Löschen und Abschalten sowie Ablehnung ungültiger Puffer. Der bestehende ConPTY-/egui-Test prüft zusätzlich Upload, Farbwerte und Zeichenprimitive eines direkt eingefügten lokalen Bildes.

Native Explorer-Ablage und Windows-Dateidialog wurden nicht im sichtbaren Fenster bedient. Die UI-Prüfung speist egui-Eingabeereignisse ein; Rendererprüfungen laufen ohne sichtbares Fenster. Keine echten Remote-Verbindungen oder Datei-Uploads.

## Kontextabhängiges Bildöffnen 0.9.1 – 22. Oktober 2026

112 Tests und die vier lokalen PTY-Modi erfolgreich; Formatierung und Release-Build bestanden. Neue Prüfung der Pfadauflösung für Verzeichnislisten, absolute/relative Namen, Leerzeichen, Unicode, Apostrophe, mehrere Dateien in einer Zeile und konkurrierende Namenssuffixe. Nicht unterstützte Endungen, angehängte EXE-Endungen und HTTP-URLs werden nicht als direkte Bildziele erkannt. Der ConPTY-Test prüft zusätzlich die Zuordnung einer Pixelposition zum korrekten UTF-8-Zeichen in ausgegebenem Terminaltext. Nach der Präzisierung der Kandidatenreihenfolge wurden Pfadtest und Release-Build erneut ausgeführt. Native Rechtsklick-Bedienung im sichtbaren Fenster bleibt manuell zu prüfen.
