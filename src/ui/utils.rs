use regex::Regex;
use std::cmp;
use terminal_size::{Width, terminal_size};

pub struct RoundedBox {
    pub text: String,
    pub title: Option<String>,
    pub color: Option<AnsiColor>,
    pub center: bool,
}

impl RoundedBox {
    pub fn new(text: &str, title: Option<&str>, color: Option<AnsiColor>, center: bool) -> Self {
        Self {
            text: text.to_string(),
            title: title.map(|t| t.to_string()),
            color,
            center,
        }
    }

    pub fn render(&self) -> String {
        let top_left = '╭';
        let top_right = '╮';
        let bottom_left = '╰';
        let bottom_right = '╯';
        let horizontal = '─';
        let vertical = if let Some(color) = &self.color {
            colorize_text('│'.to_string().as_str(), *color)
        } else {
            "│".to_string()
        };
        let space = ' ';

        let text_lines: Vec<&str> = self.text.lines().collect();
        let terminal_width = if let Some((Width(w), _)) = terminal_size() {
            w as usize
        } else {
            80
        };

        let max_line_length = cmp::min(
            text_lines
                .iter()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(0),
            terminal_width - 4,
        );
        let padding = 1;

        let wrapped_lines: Vec<String> = text_lines
            .iter()
            .flat_map(|line| {
                line.chars()
                    .collect::<Vec<_>>()
                    .chunks(max_line_length)
                    .map(|chunk| chunk.iter().collect::<String>())
                    .collect::<Vec<_>>()
            })
            .collect();

        let inner_width = max_line_length + (2 * padding);

        let title_segment = if let Some(title) = &self.title {
            let title_len = title.chars().count();
            let total_width = inner_width + 2;
            let title_padding = (total_width.saturating_sub(title_len)) / 2;
            format!(
                "{}{}{}",
                horizontal.to_string().repeat(title_padding - 2),
                format!(" {} ", title),
                horizontal
                    .to_string()
                    .repeat(total_width.saturating_sub(title_len + title_padding) - 2)
            )
        } else {
            horizontal.to_string().repeat(inner_width + 2)
        };

        let top_border = format!("{}{}{}", top_left, title_segment, top_right);

        let horizontal_padding_line = format!(
            "{}{}{}",
            vertical,
            space.to_string().repeat(inner_width),
            vertical
        );

        let escape_code_regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();

        let calculate_visible_length =
            |line: &str| escape_code_regex.replace_all(line, "").chars().count();

        let text_lines_rendered: Vec<String> = wrapped_lines
            .iter()
            .map(|line| {
                let visible_length = calculate_visible_length(line);
                if self.center {
                    let line_padding = inner_width.saturating_sub(visible_length);
                    let left_padding = line_padding / 2;
                    let right_padding = line_padding - left_padding;
                    format!(
                        "{}{}{}{}{}",
                        vertical,
                        space.to_string().repeat(left_padding),
                        line,
                        space.to_string().repeat(right_padding),
                        vertical
                    )
                } else {
                    let right_padding = inner_width.saturating_sub(visible_length);
                    format!(
                        "{}{}{}{}",
                        vertical,
                        line,
                        space.to_string().repeat(right_padding),
                        vertical
                    )
                }
            })
            .collect();

        let bottom_border = format!(
            "{}{}{}",
            bottom_left,
            horizontal.to_string().repeat(inner_width),
            bottom_right
        );

        let box_content = format!(
            "{}\n{}\n{}\n{}\n{}",
            top_border,
            horizontal_padding_line,
            text_lines_rendered.join("\n"),
            horizontal_padding_line,
            bottom_border
        );

        if let Some(color) = &self.color {
            let colored_top_border = format!("\x1b[{}m{}\x1b[0m", color.to_ansi_code(), top_border);
            let colored_bottom_border =
                format!("\x1b[{}m{}\x1b[0m", color.to_ansi_code(), bottom_border);

            format!(
                "{}\n{}\n{}\n{}\n{}",
                colored_top_border,
                horizontal_padding_line,
                text_lines_rendered.join("\n"),
                horizontal_padding_line,
                colored_bottom_border
            )
        } else {
            box_content
        }
    }
}

pub fn colorize_text(text: &str, color: AnsiColor) -> String {
    format!("\x1b[{}m{}\x1b[0m", color.to_ansi_code(), text)
}

#[derive(Copy, Clone)]
pub enum AnsiColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl AnsiColor {
    pub fn to_ansi_code(&self) -> &str {
        const ANSI_CODES: [&str; 16] = [
            "30", "31", "32", "33", "34", "35", "36", "37", // Standard colors
            "90", "91", "92", "93", "94", "95", "96", "97", // Bright colors
        ];
        ANSI_CODES[*self as usize]
    }
}
