// src/layout.rs
use crate::fb2_parser::Paragraph;
use textwrap::{fill, Options};

pub fn prepare_layout(
    paragraphs: &[Paragraph],
    width: u16,
) -> (Vec<String>, Vec<(String, usize)>) {
    let mut new_lines = Vec::new();
    let mut new_toc = Vec::new();
    let w = width as usize;

    for (_p_idx, p) in paragraphs.iter().enumerate() {
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
                let mut original_indent = 0;
                for c in text.chars() {
                    if c == ' ' { original_indent += 1; } else { break; }
                }
                
                let body_indent_size = if original_indent > 0 { original_indent } else { 4 };
                let body_indent = " ".repeat(body_indent_size);

                let t = text.trim().to_string();
                if t.is_empty() {
                    continue;
                }

                let options = Options::new(w.saturating_sub(body_indent_size));
                let wrapped = fill(&t, options);
                let lines: Vec<_> = wrapped.lines().collect();
                let len = lines.len();

                for (i, line) in lines.iter().enumerate() {
                    let formatted = if i == 0 {
                        let first_line = format!("{}{}", body_indent, line);
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

            Paragraph::Epigraph(text) => {
                let lines: Vec<&str> = text.lines().collect();
                if !lines.is_empty() {
                    let options = Options::new(w.saturating_sub(4));
                    for line in &lines {
                        let wrapped = fill(line.trim(), &options);
                        for wrapped_line in wrapped.lines() {
                            new_lines.push(format!("  {}", wrapped_line));
                        }
                    }
                }
            }

            Paragraph::Cite(text) => {
                let lines: Vec<&str> = text.lines().collect();
                if !lines.is_empty() {
                    let options = Options::new(w.saturating_sub(6));
                    for line in &lines {
                        let wrapped = fill(line.trim(), &options);
                        for wrapped_line in wrapped.lines() {
                            new_lines.push(format!("    {}", wrapped_line));
                        }
                    }
                }
            }

            Paragraph::Poem(text) => {
                let lines: Vec<&str> = text.lines().collect();
                if !lines.is_empty() {
                    for line in &lines {
                        let wrapped = fill(line.trim(), w.saturating_sub(4));
                        for wrapped_line in wrapped.lines() {
                            new_lines.push(format!("  {}", wrapped_line));
                        }
                    }
                }
            }

            Paragraph::Author(text) => {
                let wrapped = fill(text.trim(), w.saturating_sub(8));
                for line in wrapped.lines() {
                    new_lines.push(format!("    {}", line));
                }
            }

            Paragraph::Subtitle(text) => {
                let wrapped = fill(text.trim(), w.saturating_sub(8));
                for line in wrapped.lines() {
                    new_lines.push(format!("    {}", line));
                }
            }

            Paragraph::EmphasisBlock(text) => {
                let wrapped = fill(text.trim(), w.saturating_sub(8));
                for line in wrapped.lines() {
                    new_lines.push(format!("    {}", line));
                }
            }
        }
    }

    (new_lines, new_toc)
}

pub fn justify_line(line: &str, width: usize) -> String {
    let mut indent_count = 0;
    for c in line.chars() {
        if c == ' ' {
            indent_count += 1;
        } else {
            break;
        }
    }
    let indent = " ".repeat(indent_count);
    
    let text_part = line.trim();
    let words: Vec<&str> = text_part.split_whitespace().collect();
    
    if words.len() <= 1 {
        return line.to_string();
    }
    
    let total_chars: usize = words.iter().map(|w| w.chars().count()).sum();
    
    if total_chars + indent_count >= width {
        return format!("{}{}", indent, words.join(" "));
    }
    
    let total_spaces = width - total_chars - indent_count;
    let gaps = words.len() - 1;
    let space_width = total_spaces / gaps;
    let remainder = total_spaces % gaps;
    
    let mut result = String::from(&indent);
    for (i, word) in words.iter().enumerate() {
        result.push_str(word);
        if i < gaps {
            let calc_spaces = if i < remainder { space_width + 1 } else { space_width };
            let n = calc_spaces.max(1); 
            result.push_str(&" ".repeat(n));
        }
    }
    result
}
