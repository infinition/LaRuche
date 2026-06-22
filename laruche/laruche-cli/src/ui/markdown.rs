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

pub fn clean_agent_text(text: &str) -> String {
    let mut clean = text.to_string();
    
    // Strip XML blocks
    for tag in &["plan", "tool_call", "think"] {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);
        while let Some(start_idx) = clean.find(&start_tag) {
            if let Some(end_idx) = clean.find(&end_tag) {
                if end_idx > start_idx {
                    clean.replace_range(start_idx..(end_idx + end_tag.len()), "");
                    continue;
                }
            }
            // Strip from start of tag to end of text if tag is open
            clean.truncate(start_idx);
            break;
        }
    }
    
    // Strip open tags at the end of the text
    for tag in &["plan", "tool_call", "think"] {
        let start_tag = format!("<{}>", tag);
        if let Some(idx) = clean.find(&start_tag) {
            clean.truncate(idx);
        }
    }
    
    clean.trim().to_string()
}

