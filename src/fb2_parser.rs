// src/fb2_parser.rs - исправленная версия с поддержкой text-author

use anyhow::Result;
use roxmltree::{Document, Node};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use zip::ZipArchive;

#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct BookMeta {
    pub title: String,
    pub author: String,
    pub series: String,
    pub annotation: String,
    pub publish: String,
    pub sequence_number: i32,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Paragraph {
    Title(String),
    Epigraph(String),
    Cite(String),
    Poem(String),
    Subtitle(String),
    Author(String),
    Body(String),
    EmphasisBlock(String),
}

pub struct FB2Parser {
    pub paragraphs: Vec<Paragraph>,
    pub notes: HashMap<String, String>,
    pub toc: Vec<(String, usize)>,
    pub meta: BookMeta,
    pub encoding: String,
    pub footnotes_locations: Vec<(usize, String)>,
}

impl FB2Parser {
    pub fn new(filename: &PathBuf, unknown_title: &str, unknown_author: &str) -> Self {
        let mut parser = Self {
            paragraphs: Vec::new(),
            notes: HashMap::new(),
            toc: Vec::new(),
            encoding: "UTF-8".to_string(),
            footnotes_locations: Vec::new(),
            meta: BookMeta {
                title: unknown_title.to_string(),
                author: unknown_author.to_string(),
                ..Default::default()
            },
        };
        let _ = parser._load_and_parse(filename, unknown_author);
        parser
    }

    fn _load_and_parse(&mut self, filename: &PathBuf, unknown_author: &str) -> Result<()> {
        let mut raw_data = Vec::new();
        if filename.to_string_lossy().to_lowercase().ends_with(".zip") {
            let file = File::open(filename)?;
            let mut archive = ZipArchive::new(file)?;
            let fb2_name = archive
                .file_names()
                .find(|n| n.to_lowercase().ends_with(".fb2"))
                .map(|n| n.to_string());
            if let Some(name) = fb2_name {
                let mut zip_file = archive.by_name(&name)?;
                zip_file.read_to_end(&mut raw_data)?;
            }
        } else if !filename.as_os_str().is_empty() && filename.exists() {
            let mut file = File::open(filename)?;
            file.read_to_end(&mut raw_data)?;
        }
        if raw_data.is_empty() {
            return Ok(());
        }
        let (res, _encoding_used, has_errors) = encoding_rs::UTF_8.decode(&raw_data);
        let text_data = if has_errors {
            let (res_1251, _, _) = encoding_rs::WINDOWS_1251.decode(&raw_data);
            self.encoding = "CP1251".to_string();
            res_1251.into_owned()
        } else {
            self.encoding = "UTF-8".to_string();
            res.into_owned()
        };
        let doc = Document::parse(&text_data)?;
        self._extract_all(doc.root(), unknown_author);
        Ok(())
    }

    fn _extract_all(&mut self, root: Node, unknown_author: &str) {
        // --- ПАРСИМ МЕТАДАННЫЕ КНИГИ ---
        if let Some(ti) = root
            .descendants()
            .find(|n| n.tag_name().name() == "title-info")
        {
            if let Some(t_el) = ti.children().find(|n| n.tag_name().name() == "book-title") {
                self.meta.title = self._get_text_with_notes(t_el);
            }
            if let Some(auth) = ti.children().find(|n| n.tag_name().name() == "author") {
                let fn_ = auth
                    .children()
                    .find(|n| n.tag_name().name() == "first-name")
                    .and_then(|n| n.text())
                    .unwrap_or("");
                let mn_ = auth
                    .children()
                    .find(|n| n.tag_name().name() == "middle-name")
                    .and_then(|n| n.text())
                    .unwrap_or("");
                let ln_ = auth
                    .children()
                    .find(|n| n.tag_name().name() == "last-name")
                    .and_then(|n| n.text())
                    .unwrap_or("");
                let full_name = format!("{} {} {}", fn_, mn_, ln_)
                    .replace("  ", " ")
                    .trim()
                    .to_string();
                self.meta.author = if full_name.is_empty() {
                    unknown_author.to_string()
                } else {
                    full_name
                };
            }
            if let Some(ann) = ti.children().find(|n| n.tag_name().name() == "annotation") {
                self.meta.annotation = self._get_text_with_notes(ann);
            }
            if let Some(seq) = ti.children().find(|n| n.tag_name().name() == "sequence") {
                self.meta.series = seq.attribute("name").unwrap_or("").to_string();
                self.meta.sequence_number = seq
                    .attribute("number")
                    .and_then(|n| n.parse::<i32>().ok())
                    .unwrap_or(0);
            }
        }
        
        // Собираем сноски из body с name="notes"
        for body in root.descendants().filter(|n| n.tag_name().name() == "body") {
            if body.attribute("name") == Some("notes") {
                for sec in body.children().filter(|n| n.tag_name().name() == "section") {
                    if let Some(id) = sec.attribute("id") {
                        let note_text = self._get_text_with_notes(sec);
                        self.notes.insert(id.to_string(), note_text);
                    }
                }
            }
        }
        
        // Обрабатываем основной текст
        for body in root.descendants().filter(|n| n.tag_name().name() == "body") {
            if body.attribute("name") != Some("notes") {
                self._walk(body, None);
            }
        }
        
        // Создаем главу "Сноски" в конце оглавления
        if !self.notes.is_empty() {
            let footnote_title = "Сноски".to_string();
            self.paragraphs.push(Paragraph::Title(footnote_title.clone()));
            
            let mut footnote_numbers: Vec<(String, String)> = Vec::new();
            
            for (_, note_id) in &self.footnotes_locations {
                if let Some(_text) = self.notes.get(note_id) {
                    let mut found_num = None;
                    for paragraph in &self.paragraphs {
                        if let Paragraph::Body(body_text) = paragraph {
                            if let Some(start) = body_text.find("^f:[") {
                                if let Some(end) = body_text[start..].find(']') {
                                    let num_str = &body_text[start+4..start+end];
                                    if let Ok(num) = num_str.parse::<usize>() {
                                        let note_idx = self.footnotes_locations
                                            .iter()
                                            .position(|(_, id)| id == note_id)
                                            .map(|i| i + 1)
                                            .unwrap_or(0);
                                        if note_idx == num {
                                            found_num = Some(num);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    let display_num = found_num.map(|n| n.to_string()).unwrap_or_else(|| {
                        (self.footnotes_locations.iter().position(|(_, id)| id == note_id).unwrap_or(0) + 1).to_string()
                    });
                    
                    footnote_numbers.push((display_num, note_id.clone()));
                }
            }
            
            footnote_numbers.sort_by(|a, b| {
                let num_a = a.0.parse::<usize>().unwrap_or(0);
                let num_b = b.0.parse::<usize>().unwrap_or(0);
                num_a.cmp(&num_b)
            });
            
            for (num, note_id) in footnote_numbers {
                if let Some(text) = self.notes.get(&note_id) {
                    let clean_text = text.trim();
                    let display_text = if let Some(first_char) = clean_text.chars().next() {
                        if first_char.is_ascii_digit() {
                            let mut num_end = 0;
                            for (i, c) in clean_text.chars().enumerate() {
                                if !c.is_ascii_digit() && c != '.' && c != ')' && c != '(' && c != ' ' {
                                    num_end = i;
                                    break;
                                }
                            }
                            if num_end > 0 {
                                let rest = &clean_text[num_end..].trim_start();
                                format!("{}. {}", num, rest)
                            } else {
                                format!("{}. {}", num, clean_text)
                            }
                        } else {
                            format!("{}. {}", num, clean_text)
                        }
                    } else {
                        format!("{}. {}", num, clean_text)
                    };
                    
                    self.paragraphs.push(Paragraph::Body(display_text));
                }
            }
            
            let toc_index = self.paragraphs.len() - self.notes.len() - 1;
            self.toc.push((footnote_title, toc_index));
        }
    }

        fn _walk(&mut self, element: Node, current_mode: Option<&str>) {
        for child in element.children() {
            let tag = child.tag_name().name();
            let next_mode = match tag {
                "epigraph" => Some("epigraph"),
                "cite" => Some("cite"),
                "stanza" => Some("poem"),
                "subtitle" => Some("subtitle"),
                _ => current_mode,
            };
            match tag {
                "title" => {
                    let text = self._get_text_with_notes(child);
                    if !text.is_empty() {
                        if element.tag_name().name() == "section" {
                            self.toc.push((text.clone(), self.paragraphs.len()));
                        }
                        self.paragraphs.push(Paragraph::Title(text));
                    }
                }
"p" | "v" => {
    let text = self._get_text_with_notes(child);
    if !text.is_empty() {
        if text.chars().all(|c| c == '*' || c.is_whitespace())
            && text.matches('*').count() >= 3
        {
            self.paragraphs.push(Paragraph::Body(text));
            continue;
        }
        let new_paragraph = match (tag, next_mode) {
            ("p", Some("poem")) => Paragraph::Poem(text),
            ("p", Some("epigraph")) => Paragraph::Epigraph(text),
            ("p", Some("cite")) => Paragraph::Cite(text),
            ("subtitle", _) | (_, Some("subtitle")) => Paragraph::Subtitle(text),
            _ => Paragraph::Body(text),
        };
        
        // Проверяем последний параграф
        if let Some(last) = self.paragraphs.last() {
            match last {
                // Если последний был пустой строкой, заменяем его на новый параграф
                Paragraph::Body(s) if s.is_empty() => {
                    let last_idx = self.paragraphs.len() - 1;
                    self.paragraphs[last_idx] = new_paragraph;
                    continue;
                }
                _ => {}
            }
        }
        self.paragraphs.push(new_paragraph);
    }
}
                "text-author" => {
                    let text = self._get_text_with_notes(child);
                    if !text.is_empty() {
                        self.paragraphs.push(Paragraph::Author(text));
                        // НЕ добавляем пустую строку
                    }
                }
                "subtitle" => {
                    let text = self._get_text_with_notes(child);
                    if !text.is_empty() {
                        self.paragraphs.push(Paragraph::Subtitle(text));
                    }
                }
                "epigraph" | "cite" | "stanza" => {
                    self._walk(child, next_mode);
                }
                _ => {
                    self._walk(child, next_mode);
                }
            }
        }
    }

    fn _get_text_with_notes(&mut self, node: Node) -> String {
        let mut text = String::new();
        self._collect_text(node, &mut text, None);
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn _collect_text(&mut self, node: Node, text: &mut String, note_id_inherit: Option<&str>) {
        if node.is_text() {
            text.push_str(node.text().unwrap_or(""));
            return;
        }
        if node.is_element() {
            let tag = node.tag_name().name();

            if tag == "a" {
                let mut href = "";
                for attr in node.attributes() {
                    if attr.name().to_lowercase() == "href" {
                        href = attr.value();
                        break;
                    }
                }
                
                let note_id = if href.starts_with('#') {
                    &href[1..]
                } else {
                    ""
                };

                if !note_id.is_empty() && self.notes.contains_key(note_id) {
                    let current_paragraph_idx = self.paragraphs.len();
                    if note_id_inherit.is_none() {
                        self.footnotes_locations
                            .push((current_paragraph_idx, note_id.to_string()));
                    }
                    
                    let link_text = self._get_raw_text(node);
                    let display_num = if link_text.chars().all(|c| c.is_ascii_digit()) && !link_text.is_empty() {
                        link_text
                    } else {
                        let id_part = note_id.split('_').last().unwrap_or("");
                        if id_part.chars().all(|c| c.is_ascii_digit()) && !id_part.is_empty() {
                            id_part.to_string()
                        } else {
                            (self.footnotes_locations.len()).to_string()
                        }
                    };
                    
                    text.push_str(&format!(" ^f:[{}] ", display_num));
                    return;
                }
                
                for child in node.children() {
                    self._collect_text(child, text, note_id_inherit);
                }
                return;
            }
            
            // Добавляем пробелы только для p и v
            if tag == "p" || tag == "v" {
                text.push(' ');
            }
            
            // Рекурсивно обрабатываем все дочерние элементы (включая emphasis)
            for child in node.children() {
                self._collect_text(child, text, note_id_inherit);
            }
            
            if tag == "p" || tag == "v" {
                text.push(' ');
            }
        }
    }

    fn _get_raw_text(&self, node: Node) -> String {
        node.descendants()
            .filter(|n| n.is_text())
            .map(|n| n.text().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string()
    }
}
