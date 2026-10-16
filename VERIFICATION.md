# Prüfung

Stand: 16. Oktober 2026, Windows x64.

- Sieben Rust-Regressionstests bestehen: Workspace-Kompatibilität, Anbieter- und Pfadauflösung, Layout-Roundtrip, doppelte IDs und bereichsbezogene Ordnerübernahme.
- Release-Build mit GNU-Rust-Toolchain erfolgreich.
- PowerShell und CMD: Eingabe und Ausgabe über echte Pseudoterminals geprüft.
- Ordnerwechsel und Start eines neuen Bereichs im übernommenen Ordner geprüft, einschließlich Leerzeichen, Apostroph, Unicode sowie kurzen und langen Windows-Pfadnamen.
- Neustart einer laufenden Sitzung: frischer Prozesszustand und neue Ausgabe, aktueller Ordner erhalten.
- Neustart nach Prozessende und nach einem Startfehler geprüft.
- Verspätete Ereignisse alter Sitzungen werden verworfen; Nachbarbereich und Anbieter bleiben erhalten.
- Codex, Claude, Gemini und OpenAI CLI wurden in der lokalen Installation gestartet. Die Anbieterprogramme und deren Anmeldungen sind nicht Bestandteil des Repositorys.
- Windows-Kompatibilitätstransparenz wird über die Windows-API gesetzt und rückgelesen. Der Benutzer hat die sichtbare Funktion bestätigt.
- Der Benutzer hat die Ordnerübernahme im Terminal bestätigt.

Nicht praktisch geprüft: Git Bash (lokal nicht installiert), alle Grafikkarten und Windows-Versionen, umfassende Kompatibilität mit interaktiven Terminalprogrammen sowie der grafische Klickpfad der neu hinzugefügten Neustart-Schaltfläche. Die Neustartlogik wurde über echte PTYs geprüft.

Das portable Paketskript prüft die SHA256-Summe der festgeschriebenen PowerShell-ZIP vor dem Entpacken. Anbieter-CLIs benötigen eigene Installation und Anmeldung. Die Arbeitsbereichsdatei speichert keine laufenden Prozesse, Terminalausgaben oder KI-Chat-Verläufe.

