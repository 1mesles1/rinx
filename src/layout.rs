use crate::fb2_parser::Paragraph;
use std::collections::HashMap;
use textwrap::{fill, Options};

pub fn prepare_layout(
    paragraphs: &[Paragraph],
    width: u16,
) -> (Vec<String>, Vec<(String, usize)>, HashMap<usize, usize>) {
    let mut new_lines = Vec::new();
    let mut new_toc = Vec::new();
    let mut paragraph_to_line: HashMap<usize, usize> = HashMap::new();
    let w = width as usize;

    for (p_idx, p) in paragraphs.iter().enumerate() {
        // Запоминаем, с какой строки начинается этот абзац
        paragraph_to_line.insert(p_idx, new_lines.len());

        match p {
            Paragraph::Title(text) => {
                let clean_text = text.replace("^f:", "");
                if !new_lines.is_empty() && new_lines.last() != Some(&"".to_string()) {
                    new_lines.push("".to_string());
                }

                new_toc.push((clean_text.clone(), new_lines.len()));

                let title_text = clean_text.trim().to_uppercase();
                let wrapped = fill(&title_text, w);
                for line in wrapped.lines() {
                    let available_space = w.saturating_sub(1);
                    let padding = available_space.saturating_sub(line.chars().count()) / 2;
                    new_lines.push(format!("^:{}{}", " ".repeat(padding), line));
                }
                new_lines.push("".to_string());
            }

            Paragraph::Body(text) => {
                let t = text.trim().to_string();
                if !t.chars().any(|c| c.is_alphanumeric()) {
                    continue; // Пропускаем пустые абзацы, НО НЕ ЗАБЫВАЕМ про map
                }

                let options = Options::new(w);
                let wrapped = fill(&t, options);
                let lines: Vec<_> = wrapped.lines().collect();
                let len = lines.len();

                for (i, line) in lines.into_iter().enumerate() {
                    let formatted = if i == 0 {
                        let first_line = format!("  {}", line);
                        if len == 1 {
                            first_line
                        } else {
                            justify_line(&first_line, w)
                        }
                    } else if i < len - 1 {
                        justify_line(line, w)
                    } else {
                        line.to_string()
                    };

                    if !formatted.trim().is_empty() {
                        new_lines.push(formatted);
                    }
                }
            }

            _ => {
                let s = p.as_string_fallback().trim().to_string();
                if s.is_empty() {
                    continue;
                }
                let wrapped = fill(&s, w.saturating_sub(8));
                for line in wrapped.lines() {
                    new_lines.push(format!("    {}", line));
                }
                new_lines.push("".to_string());
            }
        }
    }

    (new_lines, new_toc, paragraph_to_line)
}

pub fn justify_line(line: &str, width: usize) -> String {
    let indent = if line.starts_with("  ") { "  " } else { "" };
    let text_part = line.trim();
    let words: Vec<&str> = text_part.split_whitespace().collect();
    if words.len() <= 1 {
        return line.to_string();
    }
    let indent_len = indent.chars().count();
    let total_chars: usize = words.iter().map(|w| w.chars().count()).sum();
    if total_chars + indent_len >= width {
        return line.to_string();
    }
    let total_spaces = width - total_chars - indent_len;
    let gaps = words.len() - 1;
    let space_width = total_spaces / gaps;
    let remainder = total_spaces % gaps;
    let mut result = String::from(indent);
    for (i, word) in words.iter().enumerate() {
        result.push_str(word);
        if i < gaps {
            let n = if i < remainder {
                space_width + 1
            } else {
                space_width
            };
            result.push_str(&" ".repeat(n));
        }
    }
    result
}

trait AsString {
    fn as_string_fallback(&self) -> &str;
}

impl AsString for Paragraph {
    fn as_string_fallback(&self) -> &str {
        match self {
            Paragraph::Title(s)
            | Paragraph::Body(s)
            | Paragraph::Poem(s)
            | Paragraph::Epigraph(s)
            | Paragraph::Cite(s)
            | Paragraph::Subtitle(s)
            | Paragraph::Author(s)
            | Paragraph::EmphasisBlock(s) => s,
        }
    }
}
