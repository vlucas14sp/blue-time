//! Renders the top bar indicator icon: a progress ring with the remaining
//! minutes (or a play/pause glyph) in the middle.

use gtk::cairo;

use crate::timer::{Phase, State, Timer};

/// GNOME panel icons are typically 22px; 44px covers HiDPI.
const SIZES: [i32; 2] = [22, 44];

pub fn render(timer: &Timer) -> Vec<ksni::Icon> {
    SIZES
        .iter()
        .map(|&size| render_at(timer, size))
        .collect()
}

fn phase_color(phase: Phase) -> (f64, f64, f64) {
    match phase {
        // GNOME palette: red 2, green 3, blue 2
        Phase::Focus => (0.93, 0.38, 0.31),
        Phase::ShortBreak => (0.34, 0.89, 0.54),
        Phase::LongBreak => (0.38, 0.63, 0.92),
    }
}

fn render_at(timer: &Timer, size: i32) -> ksni::Icon {
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, size, size)
        .expect("create icon surface");
    {
        let cr = cairo::Context::new(&surface).expect("create cairo context");
        let s = f64::from(size);
        let center = s / 2.0;
        let line = (s / 11.0).max(1.5);
        let radius = center - line / 2.0 - 0.5;

        // Track ring. White reads well on the GNOME top bar, which stays
        // dark in both the light and dark styles of the stock shell theme.
        cr.set_line_width(line);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.35);
        cr.arc(center, center, radius, 0.0, std::f64::consts::TAU);
        let _ = cr.stroke();

        // Progress arc, colored by phase, starting at 12 o'clock.
        let progress = timer.progress().clamp(0.0, 1.0);
        if progress > 0.0 {
            let (r, g, b) = phase_color(timer.phase());
            cr.set_source_rgb(r, g, b);
            cr.set_line_cap(cairo::LineCap::Round);
            let start = -std::f64::consts::FRAC_PI_2;
            cr.arc(
                center,
                center,
                radius,
                start,
                start + progress * std::f64::consts::TAU,
            );
            let _ = cr.stroke();
        }

        cr.set_source_rgb(1.0, 1.0, 1.0);
        match timer.state() {
            State::Idle => draw_play(&cr, s),
            State::Paused => draw_pause(&cr, s),
            State::Running => draw_minutes(&cr, s, timer.remaining()),
        }
    }
    surface_to_icon(surface, size)
}

fn draw_play(cr: &cairo::Context, s: f64) {
    let c = s / 2.0;
    let half = s * 0.16;
    cr.move_to(c - half * 0.7, c - half);
    cr.line_to(c - half * 0.7, c + half);
    cr.line_to(c + half * 1.1, c);
    cr.close_path();
    let _ = cr.fill();
}

fn draw_pause(cr: &cairo::Context, s: f64) {
    let c = s / 2.0;
    let bar_w = s * 0.1;
    let bar_h = s * 0.32;
    let gap = s * 0.07;
    cr.rectangle(c - gap - bar_w, c - bar_h / 2.0, bar_w, bar_h);
    cr.rectangle(c + gap, c - bar_h / 2.0, bar_w, bar_h);
    let _ = cr.fill();
}

fn draw_minutes(cr: &cairo::Context, s: f64, remaining: u32) {
    let minutes = remaining.div_ceil(60).min(99);
    let text = minutes.to_string();
    cr.select_font_face(
        "Cantarell",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(if text.len() > 1 { s * 0.42 } else { s * 0.48 });
    if let Ok(ext) = cr.text_extents(&text) {
        cr.move_to(
            s / 2.0 - ext.width() / 2.0 - ext.x_bearing(),
            s / 2.0 - ext.height() / 2.0 - ext.y_bearing(),
        );
        let _ = cr.show_text(&text);
    }
}

/// Convert a premultiplied native-endian cairo ARGB32 surface into the
/// network-byte-order straight ARGB expected by StatusNotifierItem.
fn surface_to_icon(mut surface: cairo::ImageSurface, size: i32) -> ksni::Icon {
    let stride = surface.stride() as usize;
    let width = size as usize;
    let height = size as usize;
    let mut data = vec![0u8; width * height * 4];
    {
        let src = surface.data().expect("borrow surface data");
        for y in 0..height {
            for x in 0..width {
                let px = u32::from_ne_bytes(
                    src[y * stride + x * 4..y * stride + x * 4 + 4]
                        .try_into()
                        .unwrap(),
                );
                let a = (px >> 24) & 0xff;
                let unmul = |c: u32| -> u32 { (c * 255).checked_div(a).unwrap_or(0).min(255) };
                let r = unmul((px >> 16) & 0xff);
                let g = unmul((px >> 8) & 0xff);
                let b = unmul(px & 0xff);
                let out = (a << 24) | (r << 16) | (g << 8) | b;
                data[(y * width + x) * 4..(y * width + x) * 4 + 4]
                    .copy_from_slice(&out.to_be_bytes());
            }
        }
    }
    ksni::Icon {
        width: size,
        height: size,
        data,
    }
}
