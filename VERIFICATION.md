# Prüfung

Stand: 16. Oktober 2026, Windows x64.

- Zehn Rust-Regressionstests bestehen: Workspace-Kompatibilität, Anbieter- und Pfadauflösung, Layout-Roundtrip, doppelte IDs, bereichsbezogene Ordnerübernahme, Prüfung der KI-Übergabeargumente und Schnellaktionen.
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

Nicht praktisch geprüft: Git Bash (lokal nicht installiert), alle Grafikkarten und Windows-Versionen, umfassende Kompatibilität mit interaktiven Terminalprogrammen sowie die grafischen Klickpfade von Neustart und KI-Kontextmenü. Die Übergabe wurde offline mit einem Testprogramm geprüft; tatsächliche Modellantworten und Anbieteranmeldungen bleiben dem interaktiven Betrieb vorbehalten.

Das portable Paketskript prüft die SHA256-Summe der festgeschriebenen PowerShell-ZIP vor dem Entpacken. Anbieter-CLIs benötigen eigene Installation und Anmeldung. Die Arbeitsbereichsdatei speichert keine laufenden Prozesse, Terminalausgaben oder KI-Chat-Verläufe.

