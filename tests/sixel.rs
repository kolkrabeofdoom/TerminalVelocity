use alacritty_terminal::{
    event::{Event, EventListener},
    index::{Column, Line},
    sixel::{decode, palette, Store},
    term::{test::TermSize, Config},
    vte::ansi::Processor,
    Term,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Events(Arc<Mutex<Vec<String>>>);
impl EventListener for Events {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(text) = event {
            self.0.lock().unwrap().push(text);
        }
    }
}

fn terminal() -> (Term<Events>, Processor, Events) {
    let events = Events::default();
    let mut term = Term::new(Config::default(), &TermSize::new(20, 5), events.clone());
    term.sixel.geometry(2, 6);
    (term, Processor::new(), events)
}
const RED: &[u8] = b"\x1bP0;1q\"1;1;8;6#1;2;100;0;0#1!8~\x1b\\";

#[test]
fn rgb_hls_repeat_raster_and_transparency() {
    let image = decode(
        b"\"1;1;3;1#1;2;100;0;0#1!2@#2;1;240;50;100#2@",
        true,
        [9; 4],
        2,
        &mut palette(),
    )
    .unwrap();
    assert_eq!((image.width, image.height), (3, 1));
    assert_eq!(image.rgba, [255, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255]);
    let image = decode(b"\"1;1;2;2#1@", true, [9; 4], 2, &mut palette()).unwrap();
    assert_eq!(&image.rgba[4..], &[0; 12]);
    let image = decode(b"\"1;1;2;2#1@", false, [9, 8, 7, 255], 2, &mut palette()).unwrap();
    assert_eq!(&image.rgba[4..8], &[9, 8, 7, 255]);
}

#[test]
fn aspect_and_color_plane_carriage_return() {
    let image = decode(
        b"#1;2;100;0;0#1@?$#2;2;0;100;0#2?A",
        true,
        [0; 4],
        2,
        &mut palette(),
    )
    .unwrap();
    assert_eq!((image.width, image.height), (2, 4));
    assert_eq!(&image.rgba[..4], &[255, 0, 0, 255]);
    assert_eq!(&image.rgba[20..24], &[0, 255, 0, 255]);
}

#[test]
fn malformed_and_oversized_images_are_bounded() {
    for data in [
        b"!1000000~".as_slice(),
        b"\"1;1;4096;4096",
        b"#256~",
        b"#1;2;101;0;0~",
        b"!12",
        b"\"11;1;1;1@",
        b"!999999999999~",
        b"\"1;1;5000;1",
        b"#1;3;0;0;0~",
    ] {
        assert!(
            decode(data, true, [0; 4], 1, &mut palette()).is_err(),
            "{data:?}"
        );
    }
    // Deterministic malformed-stream coverage without extra dependencies.
    let mut seed = 42u32;
    for _ in 0..500 {
        let data: Vec<_> = (0..128)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed >> 24) as u8
            })
            .collect();
        let _ = decode(&data, true, [0; 4], 1, &mut palette());
    }
}

#[test]
fn every_input_chunk_boundary_and_cursor_position() {
    for split in 0..=RED.len() {
        let (mut term, mut parser, _) = terminal();
        parser.advance(&mut term, &RED[..split]);
        parser.advance(&mut term, &RED[split..]);
        assert_eq!(term.sixel.active.tiles.len(), 1, "split {split}");
        assert_eq!(term.sixel.active.tiles[0].line, 0);
        assert_eq!(term.grid().cursor.point.line, Line(1));
        assert_eq!(term.grid().cursor.point.column, Column(0));
        assert_eq!(
            &term.sixel.active.tiles[0].image.rgba[..4],
            &[255, 0, 0, 255]
        );
    }
}

#[test]
fn incomplete_cancelled_and_invalid_sequences_recover_text() {
    for ending in [b"\x18".as_slice(), b"\x1a", b"\x1b[H", b"\x1bX"] {
        let (mut term, mut parser, _) = terminal();
        parser.advance(&mut term, &RED[..RED.len() - 2]);
        assert!(term.sixel.active.tiles.is_empty());
        parser.advance(&mut term, ending);
        parser.advance(&mut term, b"OK");
        assert!(term.sixel.active.tiles.is_empty());
    }
    let (mut term, mut parser, _) = terminal();
    parser.advance(&mut term, b"\x1bPq!999999999~\x1b\\OK");
    assert_eq!(term.sixel.rejected, 1);
    assert_eq!(term.grid()[Line(0)][Column(0)].c, 'O');
}

#[test]
fn text_and_erase_remove_only_covered_cells() {
    let (mut term, mut parser, _) = terminal();
    parser.advance(&mut term, RED);
    parser.advance(&mut term, b"\x1b[1;2HX");
    let tiles = &term.sixel.active.tiles;
    assert_eq!(tiles.len(), 2);
    assert_eq!((tiles[0].x, tiles[0].width), (0, 2));
    assert_eq!((tiles[1].x, tiles[1].source_x, tiles[1].width), (4, 4, 4));
    parser.advance(&mut term, b"\x1b[2K");
    assert!(term.sixel.active.tiles.is_empty());
}

#[test]
fn scrollback_alternate_screen_and_resize_lifecycle() {
    let (mut term, mut parser, _) = terminal();
    parser.advance(&mut term, RED);
    parser.advance(&mut term, b"\r\n\r\n\r\n\r\n\r\n");
    assert!(term.sixel.active.tiles[0].line < 0);
    parser.advance(&mut term, b"\x1b[?1049h");
    assert!(term.sixel.active.tiles.is_empty());
    parser.advance(&mut term, RED);
    parser.advance(&mut term, b"\x1b[?1049l");
    assert!(term.sixel.active.tiles[0].line < 0);
    parser.advance(&mut term, b"\x1b[3J");
    assert!(term.sixel.active.tiles.is_empty());
    parser.advance(&mut term, RED);
    term.resize(TermSize::new(30, 8));
    assert!(term.sixel.active.tiles.is_empty());
}

#[test]
fn queries_modes_and_disabled_support() {
    let (mut term, mut parser, events) = terminal();
    parser.advance(&mut term, b"\x1b[c\x1b[?1;1S\x1b[16t\x1b[?80$p");
    let replies = events.0.lock().unwrap().clone();
    assert!(replies.contains(&"\x1b[?62;4;22c".into()));
    assert!(replies.contains(&"\x1b[?1;0;256S".into()));
    assert!(replies.contains(&"\x1b[6;6;2t".into()));
    assert!(replies.contains(&"\x1b[?80;2$y".into()));
    parser.advance(&mut term, b"\x1b[?80h");
    parser.advance(&mut term, RED);
    assert_eq!(term.grid().cursor.point.line, Line(0));
    term.set_sixel_enabled(false);
    parser.advance(&mut term, RED);
    parser.advance(&mut term, b"\x1b[c");
    assert!(term.sixel.active.tiles.is_empty());
    assert_eq!(events.0.lock().unwrap().last().unwrap(), "\x1b[?6c");
}

#[test]
fn palette_persistence_is_opt_in() {
    let (mut term, mut parser, _) = terminal();
    parser.advance(&mut term, b"\x1b[?1070l");
    parser.advance(&mut term, RED);
    parser.advance(&mut term, b"\x1bP0;1q\"1;1;1;1#1@\x1b\\");
    assert_eq!(
        &term.sixel.active.tiles.last().unwrap().image.rgba[..4],
        &[255, 0, 0, 255]
    );
    parser.advance(&mut term, b"\x1b[?1070h");
    parser.advance(&mut term, b"\x1bP0;1q\"1;1;1;1#1@\x1b\\");
    assert_ne!(
        &term.sixel.active.tiles.last().unwrap().image.rgba[..4],
        &[255, 0, 0, 255]
    );
}

#[test]
fn partial_scroll_regions_and_old_image_eviction() {
    let image = Arc::new(decode(b"\"1;1;8;12!8~-!8~", true, [0; 4], 1, &mut palette()).unwrap());
    let mut store = Store::default();
    store.insert(image, 1, 0, 6, 5, 10);
    store.scroll(1, 4, -1, 10);
    assert_eq!(store.tiles.len(), 1);
    assert_eq!(store.tiles[0].source_y, 6);
    for _ in 0..40 {
        let image = Arc::new(decode(b"@", true, [0; 4], 1, &mut palette()).unwrap());
        store.insert(image, 0, 0, 6, 5, 10);
    }
    assert_eq!(store.tiles.len(), 32);
}
