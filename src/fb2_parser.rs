use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Paragraph {
    Title(String),
    Subtitle(String),
    Body(String),
    Poem(String),
    Epigraph(String),
    Cite(String),
    Author(String),
    EmphasisBlock(String),
}

#[derive(Debug, Clone, Default)]
pub struct BookMeta {
    pub title: String,
    pub author: String,
    pub series: String,
    pub sequence_number: i32,
    pub annotation: String,
}

#[derive(Debug, Clone)]
pub struct FootnoteInfo {
    pub text: String,
    pub number: usize,
}

pub struct FB2Parser {
    pub path: PathBuf,
    pub paragraphs: Vec<Paragraph>,
    pub meta: BookMeta,
    pub encoding: String,
    pub footnotes: Vec<FootnoteInfo>,
    pub toc: Vec<(String, usize)>,
}

impl FB2Parser {
    pub fn new(path: &Path) -> Self {
        let mut parser = Self {
            path: path.to_path_buf(),
            paragraphs: Vec::new(),
            meta: BookMeta::default(),
            encoding: "utf-8".to_string(),
            footnotes: Vec::new(),
            toc: Vec::new(),
        };
        if let Err(e) = parser.execute_parse() {
            eprintln!("Ошибка автоматического парсинга файла {:?}: {}", path, e);
        }
        parser
    }

    fn execute_parse(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::open(&self.path)?;
        let is_zip = self.path.extension().map_or(false, |ext| ext.to_string_lossy().to_lowercase() == "zip");

        let mut raw_bytes = Vec::new();
        if is_zip {
            let mut archive = zip::ZipArchive::new(file)?;
            let mut target_index = 0;
            for i in 0..archive.len() {
                if let Ok(zip_file) = archive.by_index(i) {
                    let file_name = zip_file.name().to_lowercase();
                    if file_name.ends_with(".fb2") {
                        target_index = i;
                        break;
                    }
                }
            }
            let mut zip_file = archive.by_index(target_index)?;
            std::io::Read::read_to_end(&mut zip_file, &mut raw_bytes)?;
        } else {
            std::io::Read::read_to_end(&mut file, &mut raw_bytes)?;
        };

        let mut encoding_name = "utf-8".to_string();
        let head_text = String::from_utf8_lossy(&raw_bytes[..std::cmp::min(raw_bytes.len(), 200)]).to_lowercase();
        if let Some(enc_part) = head_text.split("encoding=").nth(1) {
            if let Some(found_enc) = enc_part.split(|c| c == '"' || c == '\'').nth(1) {
                encoding_name = found_enc.trim().to_string();
            }
        }
        self.encoding = encoding_name.clone();

        let utf8_string = if encoding_name != "utf-8" {
            let label = encoding_name.as_bytes();
            let coder = encoding_rs::Encoding::for_label(label).unwrap_or(encoding_rs::UTF_8);
            let (res_cow, _, _) = coder.decode(&raw_bytes);
            res_cow.into_owned()
        } else {
            String::from_utf8_lossy(&raw_bytes).into_owned()
        };

        let mut xml_reader = Reader::from_reader(utf8_string.as_bytes());
        xml_reader.trim_text(true);

        let mut buf = Vec::new();
        let mut current_tag = String::new();

        let (mut in_author, mut in_title, mut in_epigraph, mut in_poem) = (false, false, false, false);
        let (mut in_cite, mut in_text_author, mut in_annotation, mut in_body) = (false, false, false, false);
        let (mut in_footnote, mut in_body_notes, mut in_section, mut in_text_block) = (false, false, false, false);

        let (mut current_footnote_id, mut current_footnote_text) = (String::new(), String::new());
        let (mut author_first, mut author_last, mut current_para_text) = (String::new(), String::new(), String::new());

        let mut current_section_titles: Vec<String> = Vec::new();
        let mut footnote_map: HashMap<String, String> = HashMap::new();
        let mut footnote_order: Vec<String> = Vec::new();
        let mut pending_footnote_num: Option<usize> = None;
        let mut in_link = false;

        loop {
            match xml_reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let raw_tag = String::from_utf8_lossy(e.name().as_ref()).to_string().to_lowercase();
                    let tag_name = raw_tag.split(':').last().unwrap_or(&raw_tag).to_string();
                    current_tag = tag_name.clone();

                    match tag_name.as_str() {
                        "author" => in_author = true,
                        "title" => { in_title = true; in_text_block = true; current_para_text.clear(); }
                        "epigraph" => { in_epigraph = true; in_text_block = true; }
                        "poem" => { in_poem = true; in_text_block = true; }
                        "cite" => { in_cite = true; in_text_block = true; }
                        "text-author" => in_text_author = true,
                        "annotation" => in_annotation = true,
                        "p" | "v" | "subtitle" => { current_para_text.clear(); pending_footnote_num = None; }
                        "section" => {
                            in_section = true;
                            current_section_titles.clear();
                            let mut is_footnote_section = false;
                            for attr in e.attributes().flatten() {
                                if String::from_utf8_lossy(attr.key.as_ref()).to_lowercase() == "id" {
                                    let id = String::from_utf8_lossy(&attr.value).into_owned();
                                    if in_body_notes {
                                        current_footnote_id = id; current_footnote_text.clear();
                                        in_footnote = true; is_footnote_section = true;
                                    }
                                }
                            }
                            if !is_footnote_section { in_text_block = true; }
                        }
                        "body" => {
                            let mut is_notes = false;
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                let val = String::from_utf8_lossy(&attr.value).to_lowercase();
                                if key == "name" && (val == "notes" || val == "footnotes" || val == "comments") {
                                    in_body_notes = true; is_notes = true;
                                }
                            }
                            if !is_notes { in_body = true; in_text_block = true; }
                        }
                        "a" => {
                            in_link = true;
                            let mut href = String::new();
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
                                if key == "href" || key.ends_with("href") {
                                    href = String::from_utf8_lossy(&attr.value).into_owned().replace('#', "");
                                    break;
                                }
                            }
                            if !href.is_empty() && !href.starts_with("http") {
                                if !footnote_order.contains(&href) { footnote_order.push(href.clone()); }
                                let note_num = footnote_order.iter().position(|id| id == &href).unwrap_or(0) + 1;
                                pending_footnote_num = Some(note_num);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let raw_tag = String::from_utf8_lossy(e.name().as_ref()).to_string().to_lowercase();
                    if raw_tag.split(':').last().unwrap_or(&raw_tag) == "sequence" {
                        for attr in e.attributes().flatten() {
                            match String::from_utf8_lossy(attr.key.as_ref()).to_lowercase().as_str() {
                                "name" => self.meta.series = String::from_utf8_lossy(&attr.value).into_owned(),
                                "number" => {
                                    if let Ok(num_str) = std::str::from_utf8(&attr.value) {
                                        self.meta.sequence_number = num_str.parse().unwrap_or(0);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let raw_tag = String::from_utf8_lossy(e.name().as_ref()).to_string().to_lowercase();
                    let tag_name = raw_tag.split(':').last().unwrap_or(&raw_tag).to_string();

                    match tag_name.as_str() {
                        "a" => { in_link = false; }
                        "author" => {
                            in_author = false;
                            let full_name = format!("{} {}", author_first, author_last).trim().to_string();
                            if !full_name.is_empty() && self.meta.author.is_empty() { self.meta.author = full_name; }
                        }
                        "title" => { in_title = false; in_text_block = false; }
                        "epigraph" => { in_epigraph = false; in_text_block = false; }
                        "poem" => { in_poem = false; in_text_block = false; }
                        "cite" => { in_cite = false; in_text_block = false; }
                        "text-author" => in_text_author = false,
                        "annotation" => in_annotation = false,
                        "p" | "v" | "subtitle" => {
                            let final_text = current_para_text.trim().to_string();
                            if !final_text.is_empty() {
                                if in_title {
                                    if in_section && current_section_titles.len() < 3 && !final_text.contains(&self.meta.author) { 
                                        current_section_titles.push(final_text.clone()); 
                                    }
                                    self.paragraphs.push(Paragraph::Title(final_text));
                                } else if in_epigraph { self.paragraphs.push(Paragraph::Epigraph(final_text));
                                } else if in_text_author { self.paragraphs.push(Paragraph::Author(final_text));
                                } else if in_poem { self.paragraphs.push(Paragraph::Poem(final_text));
                                } else if in_cite { self.paragraphs.push(Paragraph::Cite(final_text));
                                } else if in_body || in_text_block { self.paragraphs.push(Paragraph::Body(final_text)); }
                            }
                            current_para_text.clear(); pending_footnote_num = None;
                        }
                        "section" => {
                            in_section = false;
                            if !current_section_titles.is_empty() {
                                let title = current_section_titles.join(": ");
                                self.toc.push((title, self.paragraphs.len()));
                                current_section_titles.clear();
                            }
                            if in_footnote && !current_footnote_id.is_empty() {
                                if !footnote_map.contains_key(&current_footnote_id) { footnote_order.push(current_footnote_id.clone()); }
                                footnote_map.insert(current_footnote_id.clone(), current_footnote_text.trim().to_string());
                                current_footnote_id.clear(); current_footnote_text.clear(); in_footnote = false;
                            }
                            in_text_block = false;
                        }
                        "body" => { in_body_notes = false; in_body = false; in_text_block = false; }
                        _ => {}
                    }
                    current_tag.clear();
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape().unwrap_or_default().into_owned();
                    let cleaned_text = text.split_whitespace().collect::<Vec<&str>>().join(" ");

                    if !cleaned_text.is_empty() && cleaned_text.len() < 10000 {
                        if current_tag == "book-title" {
                            if self.meta.title.is_empty() { self.meta.title = cleaned_text; }
                        } else if in_footnote {
                            current_footnote_text.push_str(&cleaned_text); current_footnote_text.push(' ');
                        } else if in_author {
                            match current_tag.as_str() {
                                "first-name" => author_first = cleaned_text,
                                "last-name" => author_last = cleaned_text,
                                _ => {}
                            }
                        } else if in_annotation {
                            if !self.meta.annotation.is_empty() { self.meta.annotation.push(' '); }
                            self.meta.annotation.push_str(&cleaned_text);
                        } else if in_text_block || in_body {
                            if in_link {
                                if let Some(num) = pending_footnote_num {
                                    current_para_text = current_para_text.trim_end().to_string();
                                    current_para_text.push_str(&format!(" ^f:[{}]", num));
                                    pending_footnote_num = None;
                                }
                            } else {
                                if !current_para_text.is_empty() && !current_para_text.ends_with(' ') {
                                    let last_char = current_para_text.chars().last().unwrap_or(' ');
                                    let first_char = cleaned_text.chars().next().unwrap_or(' ');

                                    let last_is_alphanumeric = last_char.is_alphanumeric() || last_char == '?' || last_char == '!' || last_char == '.' || last_char == '"' || last_char == ')';
                                    let first_is_alphanumeric = first_char.is_alphanumeric();

                                    if last_is_alphanumeric && first_is_alphanumeric {
                                        current_para_text.push(' ');
                                    } else if first_char != '.' && first_char != ',' && first_char != '?' && first_char != '!' && first_char != ')' && first_char != ';' && first_char != ':' {
                                        current_para_text.push(' ');
                                    }
                                }
                                
                                current_para_text.push_str(&cleaned_text);
                                
                                if let Some(num) = pending_footnote_num {
                                    current_para_text = current_para_text.trim_end().to_string();
                                    current_para_text.push_str(&format!(" ^f:[{}]", num));
                                    pending_footnote_num = None;
                                }
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        for (order, id) in footnote_order.iter().enumerate() {
            if let Some(text) = footnote_map.get(id) {
                self.footnotes.push(FootnoteInfo {
                    text: text.clone(),
                    number: order + 1,
                });
            }
        }

        if !self.footnotes.is_empty() {
            self.paragraphs.push(Paragraph::Title("Сноски".to_string()));
            for footnote in &self.footnotes {
                self.paragraphs.push(Paragraph::Body(format!("{}. {}", footnote.number, footnote.text)));
            }
            let toc_index = self.paragraphs.len() - self.footnotes.len() - 1;
            self.toc.push(("Сноски".to_string(), toc_index));
        }

        if self.meta.title.is_empty() {
            let mut file_name = self.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Неизвестная книга".to_string());
            if file_name.to_lowercase().ends_with(".fb2") { file_name = file_name[..file_name.len() - 4].to_string(); }
            self.meta.title = file_name;
        }
        if self.meta.author.is_empty() { self.meta.author = "Неизвестный Автор".to_string(); }

        Ok(())
    }
}
