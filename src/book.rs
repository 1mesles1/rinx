// src/book.rs
use crate::fb2_parser::FB2Parser;
use crate::i18n::Language;
use crate::library::Library;
use crate::layout;
use reqwest::blocking::Client;
use std::io::Write;
use std::path::PathBuf;
use url::Url;

pub fn load_book_data(path: &PathBuf, width: u16) -> (FB2Parser, Vec<String>, Vec<(String, usize)>) {
    let parser = FB2Parser::new(path);
    let (lines, toc) = layout::prepare_layout(&parser.paragraphs, width);
    (parser, lines, toc)
}

pub fn perform_search(lines: &[String], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return vec![];
    }
    let q = query.to_lowercase();
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.to_lowercase().contains(&q))
        .map(|(idx, _)| idx)
        .collect()
}

pub fn download_book(url: &str, library: &mut Library, _lang: Language) -> Result<PathBuf, String> {
    let client = Client::new();
    let response = client.get(url).send().map_err(|e| format!("Ошибка загрузки: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Ошибка HTTP: {}", response.status()));
    }
    
    let bytes = response.bytes().map_err(|e| format!("Ошибка чтения: {}", e))?;
    
    let filename = Url::parse(url)
        .ok()
        .and_then(|u| {
            u.path_segments()
                .and_then(|segs| segs.last())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "book.fb2".to_string());
    
    let filename = if filename.ends_with(".fb2") || filename.ends_with(".zip") {
        filename
    } else {
        format!("{}.fb2", filename)
    };
    
    let library_dir = if !library.scan_paths.is_empty() {
        library.scan_paths[0].clone()
    } else {
        std::env::current_dir().unwrap_or_default()
    };
    
    std::fs::create_dir_all(&library_dir).map_err(|e| format!("Ошибка создания директории: {}", e))?;
    
    let filepath = library_dir.join(&filename);
    let mut file = std::fs::File::create(&filepath).map_err(|e| format!("Ошибка создания файла: {}", e))?;
    file.write_all(&bytes).map_err(|e| format!("Ошибка записи: {}", e))?;
    
    let parser = FB2Parser::new(&filepath);
    library.books.insert(filepath.clone(), crate::library::BookEntry {
        title: parser.meta.title.clone(),
        author: parser.meta.author.clone(),
        series: parser.meta.series.clone(),
        series_num: parser.meta.sequence_number,
        last_read_line: 0,
        bookmarks: Vec::new(),
    });
    library.save();
    
    Ok(filepath)
}
