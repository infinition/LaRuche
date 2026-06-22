use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn parse_markdown<'a>(text: &'a str) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;

    for line in text.lines() {
        if line.trim().starts_with("```") {
            in_code_block = !in_code_block;
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
            )));
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(Color::Yellow),
            )));
            continue;
        }

        // Extremely naive inline parsing for **bold**, *italic*, `code`
        let mut spans = Vec::new();
        let mut current = String::new();
        let mut chars = line.chars().peekable();
        
        let mut is_bold = false;
        let mut is_code = false;

        while let Some(c) = chars.next() {
            if c == '`' {
                if !current.is_empty() {
                    let mut style = Style::default();
                    if is_bold {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    spans.push(Span::styled(current.clone(), style));
                    current.clear();
                }
                is_code = !is_code;
                continue;
            }
            if c == '*' && chars.peek() == Some(&'*') {
                chars.next(); // consume second *
                if !current.is_empty() {
                    let mut style = Style::default();
                    if is_code {
                        style = style.fg(Color::Cyan);
                    }
                    spans.push(Span::styled(current.clone(), style));
                    current.clear();
                }
                is_bold = !is_bold;
                continue;
            }

            current.push(c);
        }

        if !current.is_empty() {
            let mut style = Style::default();
            if is_bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if is_code {
                style = style.fg(Color::Cyan);
            }
            spans.push(Span::styled(current, style));
        }

        if spans.is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(spans));
        }
    }

    lines
}
