use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Converts an ANSI-encoded string into a Ratatui `Line<'static>`.
/// Supports standard colors (30-37, 90-97), background colors (40-47, 100-107),
/// 256-color palette (38;5;n / 48;5;n), 24-bit RGB (38;2;r;g;b / 48;2;r;g;b),
/// modifiers (bold, dim, italic, underline, reversed), resets,
/// and terminal cursor manipulation / erasure (CUF, CUB, CHA, EL, ED, tabs).
pub fn parse_ansi_to_line(text: &str) -> Line<'static> {
    let mut chars: Vec<(char, Style)> = Vec::new();
    let mut cursor: usize = 0;
    let mut current_style = Style::default();

    let bytes = text.as_bytes();
    let mut i = 0;
    let len = bytes.len();

    while i < len {
        if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'[' {
            i += 2;
            let start = i;

            while i < len && !bytes[i].is_ascii_alphabetic() {
                i += 1;
            }

            if i < len {
                let cmd = bytes[i];
                let params_str = &text[start..i];
                i += 1;

                match cmd {
                    b'm' => {
                        apply_sgr_params(params_str, &mut current_style);
                    }
                    b'C' => {
                        // CUF - Cursor Forward
                        let n = if params_str.is_empty() {
                            1
                        } else {
                            params_str.parse::<usize>().unwrap_or(1)
                        };
                        while chars.len() < cursor + n {
                            chars.push((' ', Style::default()));
                        }
                        cursor += n;
                    }
                    b'D' => {
                        // CUB - Cursor Back / Left
                        let n = if params_str.is_empty() {
                            1
                        } else {
                            params_str.parse::<usize>().unwrap_or(1)
                        };
                        cursor = cursor.saturating_sub(n);
                    }
                    b'G' => {
                        // CHA - Cursor Horizontal Absolute (1-based)
                        let col = if params_str.is_empty() {
                            0
                        } else {
                            params_str.parse::<usize>().unwrap_or(1).saturating_sub(1)
                        };
                        while chars.len() < col {
                            chars.push((' ', Style::default()));
                        }
                        cursor = col;
                    }
                    b'K' | b'J' => {
                        // EL / ED - Erase in Line / Erase in Display
                        let mode = if params_str.is_empty() {
                            0
                        } else {
                            params_str.parse::<u8>().unwrap_or(0)
                        };
                        match mode {
                            0 => {
                                // Erase from cursor to end of line
                                chars.truncate(cursor);
                            }
                            1 => {
                                // Erase from beginning of line to cursor
                                for j in 0..cursor.min(chars.len()) {
                                    chars[j] = (' ', Style::default());
                                }
                            }
                            2 => {
                                // Erase entire line
                                chars.clear();
                                cursor = 0;
                            }
                            _ => {
                                chars.truncate(cursor);
                            }
                        }
                    }
                    b'@' => {
                        // ICH - Insert Character
                        let n = if params_str.is_empty() {
                            1
                        } else {
                            params_str.parse::<usize>().unwrap_or(1)
                        };
                        for _ in 0..n {
                            if cursor <= chars.len() {
                                chars.insert(cursor, (' ', Style::default()));
                            }
                        }
                    }
                    _ => {}
                }
            }
        } else if bytes[i] == b'\t' {
            let next_tab = (cursor / 8 + 1) * 8;
            while chars.len() < next_tab {
                chars.push((' ', Style::default()));
            }
            cursor = next_tab;
            i += 1;
        } else if bytes[i] == b'\r' {
            if i + 1 < len && bytes[i + 1] == b'\n' {
                i += 1;
                continue;
            }
            cursor = 0;
            i += 1;
        } else if bytes[i] == b'\n' {
            chars.push(('\n', Style::default()));
            cursor = chars.len();
            i += 1;
        } else if let Some(ch) = text[i..].chars().next() {
            if cursor < chars.len() {
                chars[cursor] = (ch, current_style);
            } else {
                chars.push((ch, current_style));
            }
            cursor += 1;
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }

    if chars.is_empty() {
        return Line::from(vec![Span::raw("")]);
    }

    // Group consecutive characters with identical style into Spans
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_span_text = String::new();
    let mut current_span_style = chars[0].1;

    for (ch, style) in chars {
        if style == current_span_style {
            current_span_text.push(ch);
        } else {
            if !current_span_text.is_empty() {
                spans.push(Span::styled(
                    std::mem::take(&mut current_span_text),
                    current_span_style,
                ));
            }
            current_span_style = style;
            current_span_text.push(ch);
        }
    }

    if !current_span_text.is_empty() {
        spans.push(Span::styled(current_span_text, current_span_style));
    }

    Line::from(spans)
}

/// Strips all ANSI escape sequences from a string, returning clean plain text,
/// while faithfully emulating cursor movements (CUF, CUB, CHA, tabs) and erasures (EL, ED).
pub fn strip_ansi(text: &str) -> String {
    let mut chars: Vec<char> = Vec::new();
    let mut cursor: usize = 0;

    let bytes = text.as_bytes();
    let mut i = 0;
    let len = bytes.len();

    while i < len {
        if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'[' {
            i += 2;
            let start = i;

            while i < len && !bytes[i].is_ascii_alphabetic() {
                i += 1;
            }

            if i < len {
                let cmd = bytes[i];
                let params_str = &text[start..i];
                i += 1;

                match cmd {
                    b'C' => {
                        let n = if params_str.is_empty() {
                            1
                        } else {
                            params_str.parse::<usize>().unwrap_or(1)
                        };
                        while chars.len() < cursor + n {
                            chars.push(' ');
                        }
                        cursor += n;
                    }
                    b'D' => {
                        let n = if params_str.is_empty() {
                            1
                        } else {
                            params_str.parse::<usize>().unwrap_or(1)
                        };
                        cursor = cursor.saturating_sub(n);
                    }
                    b'G' => {
                        let col = if params_str.is_empty() {
                            0
                        } else {
                            params_str.parse::<usize>().unwrap_or(1).saturating_sub(1)
                        };
                        while chars.len() < col {
                            chars.push(' ');
                        }
                        cursor = col;
                    }
                    b'K' | b'J' => {
                        let mode = if params_str.is_empty() {
                            0
                        } else {
                            params_str.parse::<u8>().unwrap_or(0)
                        };
                        match mode {
                            0 => chars.truncate(cursor),
                            1 => {
                                for j in 0..cursor.min(chars.len()) {
                                    chars[j] = ' ';
                                }
                            }
                            2 => {
                                chars.clear();
                                cursor = 0;
                            }
                            _ => chars.truncate(cursor),
                        }
                    }
                    b'@' => {
                        let n = if params_str.is_empty() {
                            1
                        } else {
                            params_str.parse::<usize>().unwrap_or(1)
                        };
                        for _ in 0..n {
                            if cursor <= chars.len() {
                                chars.insert(cursor, ' ');
                            }
                        }
                    }
                    _ => {}
                }
            }
        } else if bytes[i] == b'\t' {
            let next_tab = (cursor / 8 + 1) * 8;
            while chars.len() < next_tab {
                chars.push(' ');
            }
            cursor = next_tab;
            i += 1;
        } else if bytes[i] == b'\r' {
            if i + 1 < len && bytes[i + 1] == b'\n' {
                i += 1;
                continue;
            }
            cursor = 0;
            i += 1;
        } else if bytes[i] == b'\n' {
            chars.push('\n');
            cursor = chars.len();
            i += 1;
        } else if let Some(ch) = text[i..].chars().next() {
            if cursor < chars.len() {
                chars[cursor] = ch;
            } else {
                chars.push(ch);
            }
            cursor += 1;
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }

    chars.into_iter().collect()
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

    #[test]
    fn test_cursor_forward_indentation() {
        let input = "\x1b[13CUsage:";
        let line = parse_ansi_to_line(input);
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content, "             Usage:");

        let clean = strip_ansi(input);
        assert_eq!(clean, "             Usage:");
    }

    #[test]
    fn test_tab_expansion() {
        let input = "cmd\targ";
        let line = parse_ansi_to_line(input);
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content, "cmd     arg");

        let clean = strip_ansi(input);
        assert_eq!(clean, "cmd     arg");
    }

    #[test]
    fn test_zephyr_prompt_erase_and_overwrite() {
        let input = "\x1b[1;32mrtt:~$ \x1b[m\x1b[7D\x1b[J[00:00:00.015,777] <dbg> hsm: ENTRY(Advertising)";
        let line = parse_ansi_to_line(input);
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content, "[00:00:00.015,777] <dbg> hsm: ENTRY(Advertising)");

        let clean = strip_ansi(input);
        assert_eq!(clean, "[00:00:00.015,777] <dbg> hsm: ENTRY(Advertising)");
    }
}
