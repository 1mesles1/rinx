// src/fb2_parser.rs

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use anyhow::Result;
use roxmltree::{Document, Node};
use zip::ZipArchive;

#[derive(Debug, Default)]
pub struct BookMeta {
    pub title: String,
    pub author: String,
    pub series: String,
    pub annotation: String,
    pub publish: String,
}

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
}

impl FB2Parser {
    pub fn new(filename: &PathBuf, unknown_title: &str, unknown_author: &str) -> Self {
        let mut parser = Self {
            paragraphs: Vec::new(),
            notes: HashMap::new(),
            toc: Vec::new(),
            encoding: "UTF-8".to_string(),
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
            let fb2_name = archive.file_names()
                .find(|n| n.to_lowercase().ends_with(".fb2"))
                .map(|n| n.to_string());

            if let Some(name) = fb2_name {
                let mut zip_file = archive.by_name(&name)?;
                zip_file.read_to_end(&mut raw_data)?;
            }
        } else {
            let mut file = File::open(filename)?;
            file.read_to_end(&mut raw_data)?;
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
        if let Some(ti) = root.descendants().find(|n| n.tag_name().name() == "title-info") {
            if let Some(t_el) = ti.children().find(|n| n.tag_name().name() == "book-title") {
                self.meta.title = self._get_text_with_notes(t_el);
            }
            
            // Сбор автора
            if let Some(auth) = ti.children().find(|n| n.tag_name().name() == "author") {
                let fn_ = auth.children().find(|n| n.tag_name().name() == "first-name").and_then(|n| n.text()).unwrap_or("");
                let mn_ = auth.children().find(|n| n.tag_name().name() == "middle-name").and_then(|n| n.text()).unwrap_or("");
                let ln_ = auth.children().find(|n| n.tag_name().name() == "last-name").and_then(|n| n.text()).unwrap_or("");
                
                let full_name = format!("{} {} {}", fn_, mn_, ln_).replace("  ", " ").trim().to_string();
                self.meta.author = if full_name.is_empty() { unknown_author.to_string() } else { full_name };
            }

            if let Some(ann) = ti.children().find(|n| n.tag_name().name() == "annotation") {
                self.meta.annotation = self._get_text_with_notes(ann);
            }
            
            if let Some(seq) = ti.children().find(|n| n.tag_name().name() == "sequence") {
                self.meta.series = seq.attribute("name").unwrap_or("").to_string();
            }
        }

        for body in root.descendants().filter(|n| n.tag_name().name() == "body") {
            if body.attribute("name") == Some("notes") {
                for sec in body.children().filter(|n| n.tag_name().name() == "section") {
                    if let Some(id) = sec.attribute("id") {
                        self.notes.insert(id.to_string(), self._get_text_with_notes(sec));
                    }
                }
            } else {
                self._walk(body, None);
            }
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
                },
                "p" | "v" | "text-author" | "subtitle" => {
                    let text = self._get_text_with_notes(child);
                    if !text.is_empty() {
                        if text.chars().all(|c| c == '*' || c.is_whitespace()) && text.matches('*').count() >= 3 {
                            self.paragraphs.push(Paragraph::Body(text));
                            continue;
                        }

                        let new_paragraph = match (tag, next_mode) {
                            ("text-author", _) => Paragraph::Author(text),
                            ("p", Some("poem")) => Paragraph::Poem(text),
                            ("p", Some("epigraph")) => Paragraph::Epigraph(text),
                            ("p", Some("cite")) => Paragraph::Cite(text),
                            ("subtitle", _) | (_, Some("subtitle")) => Paragraph::Subtitle(text),
                            _ => Paragraph::Body(text),
                        };
                        self.paragraphs.push(new_paragraph);
                    }
                },
                "empty-line" => {
                    // Оставляем как маркер, если нужно, но layout это отфильтрует
                    self.paragraphs.push(Paragraph::Body("".to_string()));
                },
                _ => { self._walk(child, next_mode); }
            }
        }
    }

    fn _get_text_with_notes(&self, node: Node) -> String {
        node.descendants()
            .filter(|n| n.is_text())
            .map(|n| n.text().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}
