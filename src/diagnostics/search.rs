//! PTY test mode `--search-test`, built only with the `test-tools` feature.

use crate::*;

pub(super) fn search_test() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    fn wait(backend: &mut TerminalBackend, marker: &str) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let text: String = backend
                .sync()
                .grid
                .display_iter()
                .map(|cell| cell.c)
                .collect();
            if text.contains(marker) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(format!("Suchtest: Ausgabe fehlt: {marker}"));
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    }
    let base = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let (sender, _receiver) = mpsc::channel();
    let executable = Shell::Cmd.resolve(None, &base).ok_or("CMD fehlt")?;
    let (program, _) = Shell::Cmd.launch(executable, &base, false, "")?;
    let commands = format!("(for /L %i in (1,1,140) do @echo TV_SEARCH_%i literal.[x] Grüße) & echo {}WRAP_NEEDLE & echo TV_SEARCH_DONE", "X".repeat(75));
    let args = vec!["/D".into(), "/Q".into(), "/K".into(), commands];
    let mut backend = TerminalBackend::new(
        1,
        egui::Context::default(),
        sender,
        BackendSettings {
            shell: program.display().to_string(),
            args,
            working_directory: Some(base),
            env: Default::default(),
        },
    )?;
    wait(&mut backend, "TV_SEARCH_DONE")?;
    backend.set_search("literal.[x]")?;
    let (start, count, _) = backend.search_status();
    if count != 140 {
        return Err(format!("Wörtliche Suche: erwartet 140 Treffer, erhalten {count}").into());
    }
    backend.next_search_match(false);
    if backend.search_status().0 != start % count + 1 {
        return Err("Vorwärtssuche fehlerhaft".into());
    }
    backend.next_search_match(true);
    if backend.search_status().0 != start {
        return Err("Rückwärtssuche fehlerhaft".into());
    }
    for _ in 0..count {
        backend.next_search_match(false);
    }
    if backend.search_status().0 != start {
        return Err("Umlauf der Suche fehlerhaft".into());
    }
    backend.set_search("TV_SEARCH_1 ")?;
    backend.sync();
    if backend.search_status().1 != 1 || backend.last_content().grid.display_offset() == 0 {
        return Err("Treffer im zurückliegenden Verlauf nicht angesprungen".into());
    }
    if !backend
        .last_content()
        .grid
        .display_iter()
        .any(|cell| backend.search_highlight(cell.point) == Some(true))
    {
        return Err("Aktiver Treffer wird nicht hervorgehoben".into());
    }
    backend.set_search("WRAP_NEEDLE")?;
    backend.sync();
    let lines: std::collections::HashSet<_> = backend
        .last_content()
        .grid
        .display_iter()
        .filter(|cell| backend.search_highlight(cell.point).is_some())
        .map(|cell| cell.point.line.0)
        .collect();
    if backend.search_status().1 != 1 || lines.len() != 2 {
        return Err("Suche über weichen Zeilenumbruch fehlerhaft".into());
    }
    backend.set_search("Grüße")?;
    if backend.search_status().1 != 140 {
        return Err("Unicode-Suche fehlerhaft".into());
    }
    backend.set_search("LITERAL.[x]")?;
    if backend.search_status().1 != 0 {
        return Err("Groß-/Kleinschreibung wird nicht beachtet".into());
    }
    backend.set_search("TV_SEARCH_LIVE")?;
    backend.process_command(egui_term::BackendCommand::Write(
        b"echo TV_SEARCH_LIVE\r".to_vec(),
    ));
    let deadline = Instant::now() + Duration::from_secs(20);
    while backend.search_status().1 == 0 {
        backend.sync();
        if Instant::now() >= deadline {
            return Err("Neue Ausgabe wurde nicht in die Suche aufgenommen".into());
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    if backend.set_search(&"a".repeat(257)).is_ok() {
        return Err("Zu langer Suchtext akzeptiert".into());
    }
    backend.set_search("")?;
    if backend.search_status() != (0, 0, false) {
        return Err("Suche nicht zurückgesetzt".into());
    }
    std::fs::write(std::env::current_exe()?.with_file_name("search-test.txt"), "OK: echte CMD-PTY; 140 wörtliche Treffer im Verlauf; Vor-/Rückwärtssuche und Umlauf; Scrollen zu älteren Treffern und Markierung; weiche Zeilenumbrüche; Unicode; Groß-/Kleinschreibung; neue Ausgabe; Eingabelimit und Leeren der Suche. Keine KI-Anfragen gesendet.")?;
    Ok(())
}
