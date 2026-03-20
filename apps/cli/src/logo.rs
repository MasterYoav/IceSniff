fn fg(r: u8, g: u8, b: u8) -> String {
    format!("\x1b[38;2;{r};{g};{b}m")
}

fn shadow_from_color((r, g, b): (u8, u8, u8)) -> String {
    fg(
        r.saturating_sub(42),
        g.saturating_sub(44),
        b.saturating_sub(52),
    )
}

fn glyph_mask(glyph: &[&str]) -> Vec<Vec<bool>> {
    glyph
        .iter()
        .map(|row| row.chars().map(|ch| ch != ' ').collect())
        .collect()
}

fn build_letter(glyph_rows: &[&str], main_rgb: (u8, u8, u8), reset: &str) -> (Vec<String>, usize) {
    let main_color = fg(main_rgb.0, main_rgb.1, main_rgb.2);
    let shadow_color = shadow_from_color(main_rgb);

    let mask = glyph_mask(glyph_rows);
    let height = mask.len();
    let width = mask[0].len();
    let canvas_width = width + 1;
    let canvas_height = height + 1;

    let mut chars = vec![vec![' '; canvas_width]; canvas_height];
    let mut colors = vec![vec![String::new(); canvas_width]; canvas_height];

    for y in 0..height {
        for x in 0..width {
            if !mask[y][x] {
                continue;
            }

            let right_edge = x + 1 >= width || !mask[y][x + 1];
            let bottom_edge = y + 1 >= height || !mask[y + 1][x];

            if right_edge && x + 1 < canvas_width && chars[y][x + 1] == ' ' {
                chars[y][x + 1] = '│';
                colors[y][x + 1] = shadow_color.clone();
            }

            if bottom_edge && y + 1 < canvas_height && chars[y + 1][x] == ' ' {
                chars[y + 1][x] = '─';
                colors[y + 1][x] = shadow_color.clone();
            }

            if right_edge && bottom_edge && x + 1 < canvas_width && y + 1 < canvas_height {
                chars[y + 1][x + 1] = '┘';
                colors[y + 1][x + 1] = shadow_color.clone();
            }
        }
    }

    for y in 0..height {
        for x in 0..width {
            if mask[y][x] {
                chars[y][x] = '█';
                colors[y][x] = main_color.clone();
            }
        }
    }

    let rendered = (0..canvas_height)
        .map(|y| {
            let mut row = String::new();
            for x in 0..canvas_width {
                let ch = chars[y][x];
                if ch == ' ' {
                    row.push(' ');
                } else {
                    row.push_str(&colors[y][x]);
                    row.push(ch);
                    row.push_str(reset);
                }
            }
            row
        })
        .collect();

    (rendered, canvas_width)
}

pub fn render_icesniff_logo() -> Vec<String> {
    let reset = "\x1b[0m";
    let font: [(&str, [&str; 7]); 6] = [
        (
            "I",
            [
                "██████",
                "  ██  ",
                "  ██  ",
                "  ██  ",
                "  ██  ",
                "  ██  ",
                "██████",
            ],
        ),
        (
            "C",
            [
                " █████",
                "██   █",
                "██    ",
                "██    ",
                "██    ",
                "██   █",
                " █████",
            ],
        ),
        (
            "E",
            [
                "██████",
                "██    ",
                "██    ",
                "█████ ",
                "██    ",
                "██    ",
                "██████",
            ],
        ),
        (
            "S",
            [
                " █████",
                "██    ",
                "██    ",
                " ████ ",
                "    ██",
                "    ██",
                "█████ ",
            ],
        ),
        (
            "N",
            [
                "██  ██",
                "██  ██",
                "███ ██",
                "██████",
                "██ ███",
                "██  ██",
                "██  ██",
            ],
        ),
        (
            "F",
            [
                "██████",
                "██    ",
                "██    ",
                "█████ ",
                "██    ",
                "██    ",
                "██    ",
            ],
        ),
    ];

    let text = ["I", "C", "E", "S", "N", "I", "F", "F"];
    let colors = [
        (18, 76, 150),
        (28, 102, 176),
        (42, 128, 198),
        (62, 156, 216),
        (92, 186, 231),
        (128, 211, 241),
        (176, 231, 248),
        (226, 245, 252),
    ];

    let mut rendered_letters = Vec::new();
    let mut widths = Vec::new();

    for (index, ch) in text.iter().enumerate() {
        let glyph = font
            .iter()
            .find(|(name, _)| name == ch)
            .map(|(_, glyph)| glyph.as_slice())
            .expect("known glyph");
        let (letter_rows, width) = build_letter(glyph, colors[index], reset);
        rendered_letters.push(letter_rows);
        widths.push(width);
    }

    let total_rows = rendered_letters
        .iter()
        .map(std::vec::Vec::len)
        .max()
        .unwrap_or_default();

    let mut lines = vec![String::new()];
    for row_index in 0..total_rows {
        let mut parts = Vec::new();
        for (index, letter) in rendered_letters.iter().enumerate() {
            if row_index < letter.len() {
                parts.push(letter[row_index].clone());
            } else {
                parts.push(" ".repeat(widths[index]));
            }
        }
        lines.push(parts.join(" "));
    }
    lines.push(String::new());
    lines
}
