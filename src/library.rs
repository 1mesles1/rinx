// src/library.rs
use crate::fb2_parser::FB2Parser;
use crate::i18n::{I18n, Language};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum SortMode {
    Title,
    Author,
    Series,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum BorderStyle {
    Plain,
    Double,
    Rounded,
}

impl Default for BorderStyle {
    fn default() -> Self {
        BorderStyle::Rounded
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct BookEntry {
    pub title: String,
    pub author: String,
    pub series: String,
    pub series_num: i32,
    pub last_read_line: usize,
    pub bookmarks: Vec<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct Library {
    pub scan_paths: Vec<PathBuf>,
    pub last_opened_book: Option<PathBuf>,
    pub books: HashMap<PathBuf, BookEntry>,
    #[serde(with = "crate::config::theme_color_serde")]
    pub theme_color: ratatui::style::Color,
    #[serde(with = "crate::config::popup_border_color_serde")]
    pub popup_border_color: ratatui::style::Color,
    pub language: Language,
    pub main_border: BorderStyle,
    pub popup_border: BorderStyle,
    #[serde(default)]
    pub first_run: bool, // Добавляем поле
}

impl Library {
    pub fn load() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_default()
            .join("rink")
            .join("library.json");
        if let Ok(content) = std::fs::read_to_string(path) {
            serde_json::from_str(&content).unwrap_or_else(|_| Self::new())
        } else {
            Self::new()
        }
    }

    pub fn new() -> Self {
        Self {
            scan_paths: vec![std::env::current_dir().unwrap_or_default()],
            last_opened_book: None,
            books: HashMap::new(),
            theme_color: ratatui::style::Color::Cyan,
            popup_border_color: ratatui::style::Color::White,
            language: Language::Ru,
            main_border: BorderStyle::Rounded,
            popup_border: BorderStyle::Double,
            first_run: true, // Помечаем как первый запуск
        }
    }

    pub fn save(&self) {
        let dir = dirs::config_dir().unwrap_or_default().join("rink");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("library.json");
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, content);
        }
    }

    pub fn scan(&mut self) {
        self.books.retain(|path, _| path.exists());
        for path in &self.scan_paths {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let p = entry.path();
                if p.is_file() {
                    let ext = p
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if ext == "fb2" || ext == "zip" {
                        let parser = FB2Parser::new(
                            &p.to_path_buf(),
                            &I18n::t(self.language, "unknown_title"),
                            &I18n::t(self.language, "unknown_author"),
                        );
                        self.books.insert(p.to_path_buf(), BookEntry {
                            title: parser.meta.title.clone(),
                            author: parser.meta.author.clone(),
                            series: parser.meta.series.clone(),
                            series_num: parser.meta.sequence_number,
                            last_read_line: 0,
                            bookmarks: Vec::new(),
                        });
                    }
                }
            }
        }
        self.save();
    }
}
