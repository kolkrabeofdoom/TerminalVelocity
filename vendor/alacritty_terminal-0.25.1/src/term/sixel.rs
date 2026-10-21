use super::*;

impl<T: EventListener> Term<T> {
    pub fn set_sixel_enabled(&mut self, enabled: bool) {
        if !enabled {
            self.sixel.clear();
        }
        self.sixel.enabled = enabled;
    }

    pub(super) fn finish_sixel(&mut self, complete: bool) {
        let Some((data, aspect, background)) = self.sixel.pending.take() else {
            return;
        };
        if !complete || !self.sixel.enabled {
            return;
        }
        let aspect = match aspect {
            2 => 5,
            3 => 3,
            0 | 1 | 4 | 6 => 2,
            _ => 1,
        };
        let mut palette = if self.sixel.private_colors {
            crate::sixel::palette()
        } else {
            self.sixel.colors
        };
        let background_rgb = self.colors[NamedColor::Background as usize].unwrap_or(Rgb {
            r: 24,
            g: 24,
            b: 24,
        });
        let image = match crate::sixel::decode(
            &data,
            background == 1,
            [background_rgb.r, background_rgb.g, background_rgb.b, 255],
            aspect,
            &mut palette,
        ) {
            Ok(image) => Arc::new(image),
            Err(_) => {
                self.sixel.rejected += 1;
                return;
            }
        };
        if !self.sixel.private_colors {
            self.sixel.colors = palette;
        }
        self.place_image(image, background != 1);
    }

    /// Insert trusted local RGBA through the same bounded image store as SIXEL.
    pub fn display_rgba(
        &mut self,
        width: usize,
        height: usize,
        rgba: Vec<u8>,
    ) -> Result<(), &'static str> {
        if !self.sixel.enabled {
            return Err("Bildanzeige ist in den Einstellungen abgeschaltet.");
        }
        if width == 0
            || height == 0
            || width > crate::sixel::MAX_SIDE
            || height > crate::sixel::MAX_SIDE
            || width.saturating_mul(height) > crate::sixel::MAX_PIXELS
            || width.checked_mul(height).and_then(|n| n.checked_mul(4)) != Some(rgba.len())
        {
            return Err("Ungültige oder zu große Bilddaten.");
        }
        let image = Arc::new(crate::sixel::Image {
            id: crate::sixel::next_image_id(),
            width,
            height,
            rgba,
        });
        let scrolling = self.sixel.scrolling;
        self.sixel.scrolling = true;
        if self.grid.cursor.point.column.0 > 0 {
            self.linefeed();
        }
        self.carriage_return();
        self.place_image(image, true);
        self.sixel.scrolling = scrolling;
        self.scroll_display(crate::grid::Scroll::Bottom);
        Ok(())
    }

    fn place_image(&mut self, image: Arc<crate::sixel::Image>, replace: bool) {
        let cell_width = self.sixel.cell_width;
        let cell_height = self.sixel.cell_height;
        let rows = image.height.div_ceil(cell_height);
        let x = if self.sixel.scrolling {
            self.grid.cursor.point.column.0 * cell_width
        } else {
            0
        };
        let line = if self.sixel.scrolling {
            for _ in 1..rows {
                self.linefeed();
            }
            self.grid.cursor.point.line.0 - (rows as i32 - 1)
        } else {
            0
        };
        let screen_lines = self.screen_lines();
        let history = self.history_size();
        let columns = self.columns();
        // Images occupy the covered cells. Subsequent text erases only those cells.
        for row in line.max(-(history as i32))..(line + rows as i32).min(screen_lines as i32) {
            for col in (x / cell_width)..(x + image.width).div_ceil(cell_width).min(columns) {
                self.grid[Line(row)][Column(col)] = Cell::default();
            }
        }
        if replace {
            self.sixel
                .active
                .erase(line, line + rows as i32 - 1, x, x + image.width);
        }
        self.sixel
            .active
            .insert(image, line, x, cell_height, screen_lines, history);
        if self.sixel.scrolling {
            self.carriage_return();
            self.linefeed();
        }
        self.mark_fully_damaged();
        self.event_proxy.send_event(Event::Wakeup);
    }
}
