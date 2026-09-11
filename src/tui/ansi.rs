use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Converts an ANSI-encoded string into a Ratatui `Line<'static>`.
/// Supports standard colors (30-37, 90-97), background colors (40-47, 100-107),
/// 256-color palette (38;5;n / 48;5;n), 24-bit RGB (38;2;r;g;b / 48;2;r;g;b),
/// modifiers (bold, dim, italic, underline, reversed), and resets.
/// Non-SGR escape sequences (e.g. cursor moves, clear line) are cleanly ignored.
pub fn parse_ansi_to_line(text: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_style = Style::default();
    let mut current_chunk = String::new();

    let bytes = text.as_bytes();
    let mut i = 0;
    let len = bytes.len();

    while i < len {
        if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'[' {
            // Flush current accumulated text chunk before style change
            if !current_chunk.is_empty() {
                spans.push(Span::styled(
                    std::mem::take(&mut current_chunk),
                    current_style,
                ));
            }

            // Move past '\x1b['
            i += 2;
            let start = i;

            // Find the final command byte of the escape sequence (e.g., 'm', 'K', 'H')
            while i < len && !bytes[i].is_ascii_alphabetic() {
                i += 1;
            }

            if i < len {
                let cmd = bytes[i];
                let params_str = &text[start..i];
                i += 1; // Move past command byte

                // 'm' denotes SGR (Select Graphic Rendition)
                if cmd == b'm' {
                    apply_sgr_params(params_str, &mut current_style);
                }
                // Other escape codes (like 'K' for clear line) are silently stripped
            }
        } else if bytes[i] == b'\r' {
            // Ignore carriage return
            i += 1;
        } else {
            // Accumulate regular UTF-8 character
            if let Some(ch) = text[i..].chars().next() {
                current_chunk.push(ch);
                i += ch.len_utf8();
            } else {
                i += 1;
            }
        }
    }

    if !current_chunk.is_empty() {
        spans.push(Span::styled(current_chunk, current_style));
    }

    if spans.is_empty() {
        Line::from(vec![Span::raw("")])
    } else {
        Line::from(spans)
    }
}

/// Strips all ANSI escape sequences from a string, returning clean plain text.
pub fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    let len = bytes.len();

    while i < len {
        if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'[' {
            i += 2;
            while i < len && !bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i < len {
                i += 1;
            }
        } else if bytes[i] == b'\r' {
            i += 1;
        } else if let Some(ch) = text[i..].chars().next() {
            result.push(ch);
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }

    result
}

fn apply_sgr_params(params_str: &str, style: &mut Style) {
    if params_str.is_empty() {
        *style = Style::default();
        return;
    }

    let mut numbers = Vec::new();
    for part in params_str.split(';') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            numbers.push(0);
        } else if let Ok(n) = trimmed.parse::<u8>() {
            numbers.push(n);
        }
    }

    if numbers.is_empty() {
        *style = Style::default();
        return;
    }

    let mut idx = 0;
    while idx < numbers.len() {
        let code = numbers[idx];
        match code {
            0 => {
                *style = Style::default();
            }
            1 => {
                *style = style.add_modifier(Modifier::BOLD);
            }
            2 => {
                *style = style.add_modifier(Modifier::DIM);
            }
            3 => {
                *style = style.add_modifier(Modifier::ITALIC);
            }
            4 => {
                *style = style.add_modifier(Modifier::UNDERLINED);
            }
            7 => {
                *style = style.add_modifier(Modifier::REVERSED);
            }
            22 => {
                *style = style.remove_modifier(Modifier::BOLD | Modifier::DIM);
            }
            23 => {
                *style = style.remove_modifier(Modifier::ITALIC);
            }
            24 => {
                *style = style.remove_modifier(Modifier::UNDERLINED);
            }
            27 => {
                *style = style.remove_modifier(Modifier::REVERSED);
            }

            // Standard Foreground
            30 => *style = style.fg(Color::Black),
            31 => *style = style.fg(Color::Red),
            32 => *style = style.fg(Color::Green),
            33 => *style = style.fg(Color::Yellow),
            34 => *style = style.fg(Color::Blue),
            35 => *style = style.fg(Color::Magenta),
            36 => *style = style.fg(Color::Cyan),
            37 => *style = style.fg(Color::Gray),
            39 => *style = style.fg(Color::Reset),

            // Standard Background
            40 => *style = style.bg(Color::Black),
            41 => *style = style.bg(Color::Red),
            42 => *style = style.bg(Color::Green),
            43 => *style = style.bg(Color::Yellow),
            44 => *style = style.bg(Color::Blue),
            45 => *style = style.bg(Color::Magenta),
            46 => *style = style.bg(Color::Cyan),
            47 => *style = style.bg(Color::Gray),
            49 => *style = style.bg(Color::Reset),

            // High Intensity Foreground
            90 => *style = style.fg(Color::DarkGray),
            91 => *style = style.fg(Color::LightRed),
            92 => *style = style.fg(Color::LightGreen),
            93 => *style = style.fg(Color::LightYellow),
            94 => *style = style.fg(Color::LightBlue),
            95 => *style = style.fg(Color::LightMagenta),
            96 => *style = style.fg(Color::LightCyan),
            97 => *style = style.fg(Color::White),

            // High Intensity Background
            100 => *style = style.bg(Color::DarkGray),
            101 => *style = style.bg(Color::LightRed),
            102 => *style = style.bg(Color::LightGreen),
            103 => *style = style.bg(Color::LightYellow),
            104 => *style = style.bg(Color::LightBlue),
            105 => *style = style.bg(Color::LightMagenta),
            106 => *style = style.bg(Color::LightCyan),
            107 => *style = style.bg(Color::White),

            // Extended Foreground (38;5;n or 38;2;r;g;b)
            38 => {
                if idx + 2 < numbers.len() && numbers[idx + 1] == 5 {
                    let color_idx = numbers[idx + 2];
                    *style = style.fg(Color::Indexed(color_idx));
                    idx += 2;
                } else if idx + 4 < numbers.len() && numbers[idx + 1] == 2 {
                    let r = numbers[idx + 2];
                    let g = numbers[idx + 3];
                    let b = numbers[idx + 4];
                    *style = style.fg(Color::Rgb(r, g, b));
                    idx += 4;
                }
            }

            // Extended Background (48;5;n or 48;2;r;g;b)
            48 => {
                if idx + 2 < numbers.len() && numbers[idx + 1] == 5 {
                    let color_idx = numbers[idx + 2];
                    *style = style.bg(Color::Indexed(color_idx));
                    idx += 2;
                } else if idx + 4 < numbers.len() && numbers[idx + 1] == 2 {
                    let r = numbers[idx + 2];
                    let g = numbers[idx + 3];
                    let b = numbers[idx + 4];
                    *style = style.bg(Color::Rgb(r, g, b));
                    idx += 4;
                }
            }

            _ => {}
        }
        idx += 1;
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_plain_text_no_ansi() {
        let line = parse_ansi_to_line("Hello World");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content, "Hello World");
        assert_eq!(line.spans[0].style, Style::default());
    }

    #[test]
    fn test_standard_colors_and_bold() {
        let input = "\x1b[1;32m[INFO]\x1b[0m System ready";
        let line = parse_ansi_to_line(input);
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].content, "[INFO]");
        assert_eq!(
            line.spans[0].style,
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        );
        assert_eq!(line.spans[1].content, " System ready");
        assert_eq!(line.spans[1].style, Style::default());
    }

    #[test]
    fn test_bright_colors() {
        let input = "\x1b[91mError: \x1b[93mCode 404\x1b[0m";
        let line = parse_ansi_to_line(input);
        assert_eq!(line.spans.len(), 2);
        assert_eq!(line.spans[0].content, "Error: ");
        assert_eq!(line.spans[0].style.fg, Some(Color::LightRed));
        assert_eq!(line.spans[1].content, "Code 404");
        assert_eq!(line.spans[1].style.fg, Some(Color::LightYellow));
    }

    #[test]
    fn test_256_color_and_rgb() {
        let input = "\x1b[38;5;42mColor42\x1b[0m \x1b[38;2;10;20;30mRGB\x1b[0m";
        let line = parse_ansi_to_line(input);
        assert_eq!(line.spans.len(), 3);
        assert_eq!(line.spans[0].content, "Color42");
        assert_eq!(line.spans[0].style.fg, Some(Color::Indexed(42)));
        assert_eq!(line.spans[1].content, " ");
        assert_eq!(line.spans[2].content, "RGB");
        assert_eq!(line.spans[2].style.fg, Some(Color::Rgb(10, 20, 30)));
    }

    #[test]
    fn test_strip_ansi() {
        let input = "\x1b[1;31m<error>\x1b[0m Failed at \x1b[4mline 42\x1b[0m\r\n";
        let clean = strip_ansi(input);
        assert_eq!(clean, "<error> Failed at line 42\n");
    }
}
