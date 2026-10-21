use alacritty_terminal::{
    event::WindowSize,
    tty::{self, EventedReadWrite},
};
use std::{
    io::Read,
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires Windows ConPTY and PowerShell 7"]
fn conpty_passes_sixel_bytes() {
    let options = tty::Options {
        shell: Some(tty::Shell::new("pwsh.exe".into(), vec!["-NoLogo".into(), "-NoProfile".into(), "-Command".into(),
            "[Console]::Write(\"`eP0;1q#1;2;100;0;0#1!12~-`e\\SIXEL_END\"); Start-Sleep -Milliseconds 500".into()])),
        escape_args: true,
        ..Default::default()
    };
    let mut pty = tty::new(
        &options,
        WindowSize {
            num_lines: 24,
            num_cols: 80,
            cell_width: 9,
            cell_height: 18,
        },
        0,
    )
    .unwrap();
    let mut output = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        let mut buffer = [0; 8192];
        match pty.reader().read(&mut buffer) {
            Ok(n) => output.extend_from_slice(&buffer[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }
        if output.windows(9).any(|s| s == b"SIXEL_END") {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        output
            .windows(25)
            .any(|s| s == b"\x1bP0;1q#1;2;100;0;0#1!12~-"),
        "raw output: {:?}",
        String::from_utf8_lossy(&output)
    );
}

#[test]
#[ignore = "requires Windows ConPTY and PowerShell 7"]
fn conpty_backend_uploads_and_paints_sixel_texture() {
    use egui_term::{BackendSettings, TerminalBackend, TerminalView};
    let context = egui::Context::default();
    let (sender, _receiver) = std::sync::mpsc::channel();
    let mut backend = TerminalBackend::new(4242, context.clone(), sender, BackendSettings {
        shell: "pwsh.exe".into(),
        args: vec!["-NoLogo".into(), "-NoProfile".into(), "-Command".into(),
            "Start-Sleep -Seconds 2; [Console]::Write(\"`eP0;1q`\"1;1;12;6#1;2;100;0;0#1!12~`e\\\"); Start-Sleep -Seconds 2".into()],
        working_directory: None,
    }).unwrap();
    let frame = |backend: &mut TerminalBackend| {
        context.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(640.0, 400.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let view = TerminalView::new(ui, backend);
                    ui.add(view);
                });
            },
        )
    };
    // Establish the actual renderer cell geometry before the child emits its image.
    let _ = frame(&mut backend);
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline && backend.sync().sixel.is_empty() {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(backend.sixel_status(), (1, 0));
    let output = frame(&mut backend);
    let (texture_id, delta) = output
        .textures_delta
        .set
        .iter()
        .find(|(_, delta)| delta.image.size() == [12, 6])
        .expect("Sixel texture upload");
    let egui::ImageData::Color(pixels) = &delta.image else {
        panic!("expected RGBA image");
    };
    assert!(pixels.pixels.iter().all(|c| *c == egui::Color32::RED));
    let primitives = context.tessellate(output.shapes, output.pixels_per_point);
    assert!(primitives.iter().any(|p| matches!(&p.primitive, egui::epaint::Primitive::Mesh(mesh) if mesh.texture_id == *texture_id)));
    backend.set_sixel_enabled(false);
    let cleared = frame(&mut backend);
    assert!(cleared.textures_delta.free.contains(texture_id));
}
