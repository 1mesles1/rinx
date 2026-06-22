// src/handlers.rs
use crate::app::{App, AppState};
use crate::book::{download_book, load_book_data, perform_search};
use crate::i18n::{I18n, Language};
use crate::library::BookEntry;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use ratatui::style::Color;
use std::path::PathBuf;

pub fn handle_key_event(
    key: KeyEvent,
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> anyhow::Result<()> {
    let lang = app.library.language;

    // Перехват клавиш при открытом окне сноски
    if app.show_footnote {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let line_count = app.current_footnote_text
                    .split('\n')
                    .count();
                let max_scroll = line_count.saturating_sub(10);
                if app.current_footnote_scroll < max_scroll {
                    app.current_footnote_scroll += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.current_footnote_scroll = app.current_footnote_scroll.saturating_sub(1);
            }
            _ => {
                app.show_footnote = false;
            }
        }
        return Ok(());
    }

    match key.code {
        // --- ВЫХОД ИЗ БИБЛИОТЕКИ ---
        KeyCode::Char('q') if matches!(app.state, AppState::Library) && !app.is_searching => {
            app.search_library_query.clear();
            app.state = AppState::Reader;
        }
        KeyCode::Esc if matches!(app.state, AppState::Library) && app.is_searching => {
            app.is_searching = false;
            app.search_library_query.clear();
        }
        KeyCode::Char('/') if matches!(app.state, AppState::Library) && !app.is_searching => {
            app.is_searching = true;
            app.search_library_query.clear();
        }
        KeyCode::Char(c) if matches!(app.state, AppState::Library) && app.is_searching => {
            app.search_library_query.push(c);
            app.library_index = 0;
        }
        KeyCode::Backspace if matches!(app.state, AppState::Library) && app.is_searching => {
            app.search_library_query.pop();
        }
        KeyCode::Enter if matches!(app.state, AppState::Library) && app.is_searching => {
            app.is_searching = false;
        }
        KeyCode::Char('s') if matches!(app.state, AppState::Library) => {
            handle_library_sort(app);
        }
        KeyCode::Home if matches!(app.state, AppState::Library) => app.library_index = 0,
        KeyCode::End if matches!(app.state, AppState::Library) => {
            app.library_index = app.library_results.len().saturating_sub(1)
        }
        KeyCode::PageUp | KeyCode::Left if matches!(app.state, AppState::Library) => {
            app.library_index = app.library_index.saturating_sub(10)
        }
        KeyCode::PageDown | KeyCode::Right if matches!(app.state, AppState::Library) => {
            app.library_index = (app.library_index + 10)
                .min(app.library_results.len().saturating_sub(1))
        }
        KeyCode::Up | KeyCode::Char('k') if matches!(app.state, AppState::Library) => {
            app.library_index = app.library_index.saturating_sub(1)
        }
        KeyCode::Down | KeyCode::Char('j') if matches!(app.state, AppState::Library) => {
            if !app.library_results.is_empty() {
                app.library_index = (app.library_index + 1)
                    .min(app.library_results.len().saturating_sub(1));
            }
        }
        KeyCode::Enter if matches!(app.state, AppState::Library) && !app.is_searching => {
            handle_library_open(app, lang)?;
        }

        // --- НАСТРОЙКИ ---
        KeyCode::Esc | KeyCode::Char('q') if matches!(app.state, AppState::Config) => {
            app.state = AppState::Reader;
        }
        KeyCode::Up | KeyCode::Char('k') if matches!(app.state, AppState::Config) => {
            app.config_index = app.config_index.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if matches!(app.state, AppState::Config) => {
            if app.config_index < 7 {
                app.config_index += 1;
            }
        }
        KeyCode::Enter if matches!(app.state, AppState::Config) => {
            handle_settings_enter(app, terminal, lang)?;
        }

        // --- ВВОД ПУТИ ---
        KeyCode::Enter if matches!(app.state, AppState::InputPath) => {
            handle_input_path_enter(app, lang);
        }
        KeyCode::Esc | KeyCode::Char('q') if matches!(app.state, AppState::InputPath) => {
            app.state = AppState::Config;
            app.input_buffer.clear();
        }
        KeyCode::Backspace if matches!(app.state, AppState::InputPath) => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) if matches!(app.state, AppState::InputPath) => {
            if app.input_buffer.starts_with("ОШИБКА") || app.input_buffer.starts_with("ERROR") {
                app.input_buffer.clear();
            }
            app.input_buffer.push(c);
        }

        // --- ВВОД ССЫЛКИ ---
        KeyCode::Enter if matches!(app.state, AppState::InputUrl) => {
            handle_input_url_enter(app, lang)?;
        }
        KeyCode::Esc | KeyCode::Char('q') if matches!(app.state, AppState::InputUrl) => {
            app.state = AppState::Config;
            app.input_buffer.clear();
        }
        KeyCode::Backspace if matches!(app.state, AppState::InputUrl) => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) if matches!(app.state, AppState::InputUrl) => {
            if app.input_buffer.starts_with("Ошибка") || app.input_buffer.starts_with("Error") {
                app.input_buffer.clear();
            }
            app.input_buffer.push(c);
        }

        // --- ПОИСК В ТЕКСТЕ ---
        KeyCode::Enter if app.is_searching && matches!(app.state, AppState::Reader) => {
            app.is_searching = false;
        }
        KeyCode::Esc | KeyCode::Char('q') if app.is_searching && matches!(app.state, AppState::Reader) => {
            app.is_searching = false;
            app.search_query.clear();
            app.search_results.clear();
        }
        KeyCode::Backspace if app.is_searching && matches!(app.state, AppState::Reader) => {
            app.search_query.pop();
            app.search_results = perform_search(&app.lines, &app.search_query);
        }
        KeyCode::Char(c) if app.is_searching && matches!(app.state, AppState::Reader) => {
            app.search_query.push(c);
            app.search_results = perform_search(&app.lines, &app.search_query);
            if !app.search_results.is_empty() {
                let start_from = app
                    .search_results
                    .iter()
                    .position(|&idx| idx >= app.scroll)
                    .unwrap_or(0);
                app.current_search_idx = start_from;
                app.scroll = app.search_results[app.current_search_idx];
            }
        }

        // --- ЗАКЛАДКИ ---
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('M') if matches!(app.state, AppState::Bookmarks) => {
            app.state = AppState::Reader;
        }
        KeyCode::Up | KeyCode::Char('k') if matches!(app.state, AppState::Bookmarks) => {
            app.library_index = app.library_index.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') if matches!(app.state, AppState::Bookmarks) => {
            if let Some(book) = app.library.books.get(&app.filename) {
                if !book.bookmarks.is_empty() {
                    app.library_index =
                        (app.library_index + 1).min(book.bookmarks.len() - 1);
                }
            }
        }
        KeyCode::Enter if matches!(app.state, AppState::Bookmarks) => {
            if let Some(book) = app.library.books.get(&app.filename) {
                if let Some(&line_idx) = book.bookmarks.get(app.library_index) {
                    app.scroll = line_idx;
                    app.state = AppState::Reader;
                }
            }
        }
        KeyCode::Char('d') | KeyCode::Delete if matches!(app.state, AppState::Bookmarks) => {
            if let Some(book) = app.library.books.get_mut(&app.filename) {
                if !book.bookmarks.is_empty()
                    && app.library_index < book.bookmarks.len()
                {
                    book.bookmarks.remove(app.library_index);
                    app.library_index = app
                        .library_index
                        .min(book.bookmarks.len().saturating_sub(1));
                    app.library.save();
                }
            }
        }

        // --- ОСНОВНЫЕ КЛАВИШИ ---
        KeyCode::Char('q') if matches!(app.state, AppState::Reader)
            && !app.show_help && !app.show_info && !app.show_toc => {
            let book = app
                .library
                .books
                .entry(app.filename.clone())
                .or_insert(BookEntry::default());
            book.last_read_line = app.scroll;
            app.library.last_opened_book = Some(app.filename.clone());
            app.library.save();
            app.should_quit = true;
        }
        KeyCode::Char('L') if !app.is_searching && matches!(app.state, AppState::Reader) => {
            app.state = AppState::Library;
            app.library_results = app.library.books.keys().cloned().collect();
            app.library_results.sort_by_key(|p| {
                app.library.books.get(p).map(|b| b.title.to_lowercase())
            });
            app.sort_mode = crate::library::SortMode::Title;
            if let Some(pos) =
                app.library_results.iter().position(|p| p == &app.filename)
            {
                app.library_index = pos;
            } else {
                app.library_index = 0;
            }
        }
        KeyCode::Char('o') if matches!(app.state, AppState::Reader) => {
            app.state = AppState::Config;
            app.config_index = 0;
        }
        KeyCode::Char('/') if matches!(app.state, AppState::Reader) && !app.is_searching => {
            app.is_searching = true;
            app.search_query.clear();
        }
        KeyCode::Char('?') if matches!(app.state, AppState::Reader) => {
            app.show_help = !app.show_help;
            app.library_index = 0;
            app.show_info = false;
            app.show_toc = false;
        }
        KeyCode::Down | KeyCode::Char('j') if app.show_help => {
            if app.library_index < 15 {
                app.library_index += 1;
            }
        }
        KeyCode::Up | KeyCode::Char('k') if app.show_help => {
            app.library_index = app.library_index.saturating_sub(1);
        }
        KeyCode::Char('i') if matches!(app.state, AppState::Reader) => {
            app.show_info = !app.show_info;
            app.show_toc = false;
        }
        KeyCode::Char('t') if matches!(app.state, AppState::Reader) => {
            app.show_toc = !app.show_toc;
            app.show_info = false;
        }
        KeyCode::Char('q') if app.show_toc => app.show_toc = false,
        KeyCode::Enter if app.show_toc => {
            if let Some((_, line_idx)) = app.toc.get(app.toc_index) {
                app.scroll = *line_idx;
                app.show_toc = false;
            }
        }
        KeyCode::Down | KeyCode::Char('j') if app.show_toc => {
            if !app.toc.is_empty() {
                app.toc_index = (app.toc_index + 1).min(app.toc.len() - 1);
            }
        }
        KeyCode::Up | KeyCode::Char('k') if app.show_toc => {
            app.toc_index = app.toc_index.saturating_sub(1);
        }
        KeyCode::Home if app.show_toc => app.toc_index = 0,
        KeyCode::End if app.show_toc => {
            app.toc_index = app.toc.len().saturating_sub(1)
        }
        KeyCode::PageUp | KeyCode::Left if app.show_toc => {
            app.toc_index = app.toc_index.saturating_sub(10)
        }
        KeyCode::PageDown | KeyCode::Right if app.show_toc => {
            app.toc_index =
                (app.toc_index + 10).min(app.toc.len().saturating_sub(1))
        }
        KeyCode::Char('n') if !app.search_results.is_empty() => {
            app.current_search_idx =
                (app.current_search_idx + 1) % app.search_results.len();
            app.scroll = app.search_results[app.current_search_idx];
        }
        KeyCode::Char('N') if !app.search_results.is_empty() => {
            if app.current_search_idx == 0 {
                app.current_search_idx = app.search_results.len() - 1;
            } else {
                app.current_search_idx -= 1;
            }
            app.scroll = app.search_results[app.current_search_idx];
        }

        // --- НАВИГАЦИЯ ---
        KeyCode::Down | KeyCode::Char('j')
            if !app.show_toc && !app.show_help && !app.show_info
                && matches!(app.state, AppState::Reader) =>
        {
            if app.scroll < app.lines.len().saturating_sub(1) {
                app.scroll += 1;
            }
        }
        KeyCode::Up | KeyCode::Char('k')
            if !app.show_toc && !app.show_help && !app.show_info
                && matches!(app.state, AppState::Reader) =>
        {
            if app.scroll > 0 {
                app.scroll -= 1;
            }
        }
        KeyCode::Left | KeyCode::PageUp
            if !app.show_toc && !app.show_help && !app.show_info
                && matches!(app.state, AppState::Reader) =>
        {
            let v_height = terminal.size()?.height.saturating_sub(3) as usize;
            app.scroll = app.scroll.saturating_sub(v_height);
        }
        KeyCode::Right | KeyCode::PageDown | KeyCode::Char(' ')
            if !app.show_toc && !app.show_help && !app.show_info
                && matches!(app.state, AppState::Reader) =>
        {
            let v_height = terminal.size()?.height.saturating_sub(3) as usize;
            let max_scroll = app.lines.len().saturating_sub(v_height);
            app.scroll = (app.scroll + v_height).min(max_scroll);
        }
        KeyCode::Home if !app.show_toc && !app.show_help && !app.show_info
            && matches!(app.state, AppState::Reader) =>
        {
            app.scroll = 0
        }
        KeyCode::End if !app.show_toc && !app.show_help && !app.show_info
            && matches!(app.state, AppState::Reader) =>
        {
            let v_height = terminal.size()?.height.saturating_sub(3) as usize;
            app.scroll = app.lines.len().saturating_sub(v_height);
        }
        KeyCode::Char('c') if matches!(app.state, AppState::Reader) => {
            app.library.theme_color = match app.library.theme_color {
                Color::Cyan => Color::Green,
                Color::Green => Color::Magenta,
                Color::Magenta => Color::Yellow,
                Color::Yellow => Color::Red,
                Color::Red => Color::White,
                _ => Color::Cyan,
            };
            if app.library.popup_border_color != Color::White {
                app.library.popup_border_color = app.library.theme_color;
            }
            app.library.save();
        }

        // --- СНОСКИ ---
        KeyCode::Char('f')
            if matches!(app.state, AppState::Reader)
                && !app.show_toc && !app.show_help && !app.show_info =>
        {
            handle_footnote_open(app, terminal)?;
        }

        // --- ЗАКЛАДКИ ---
        KeyCode::Char('m') if matches!(app.state, AppState::Reader) => {
            if let Some(book) = app.library.books.get_mut(&app.filename) {
                if !book.bookmarks.contains(&app.scroll) {
                    book.bookmarks.push(app.scroll);
                    book.bookmarks.sort();
                    app.library.save();
                }
            }
        }
        KeyCode::Char('M') if matches!(app.state, AppState::Reader) => {
            app.state = AppState::Bookmarks;
        }

        // --- ШИРИНА ---
        KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char('-')
            if matches!(app.state, AppState::Reader) =>
        {
            if key.code == KeyCode::Char('-') {
                app.width = app.width.saturating_sub(5).max(30);
            } else {
                app.width = (app.width + 5).min(100);
            }
            app.width_cache = 0;
        }

        // --- ЗАКРЫТИЕ ОКОН ---
        KeyCode::Esc | KeyCode::Char('q')
            if app.show_help || app.show_info || app.show_toc =>
        {
            app.show_help = false;
            app.show_info = false;
            app.show_toc = false;
        }
        KeyCode::Esc | KeyCode::Char('q') if !app.search_results.is_empty() && matches!(app.state, AppState::Reader) => {
            app.search_query.clear();
            app.search_results.clear();
        }
        _ => {}
    }
    Ok(())
}

// ---- ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ ОБРАБОТКИ ----

fn handle_library_sort(app: &mut App) {
    let current_path = app.library_results.get(app.library_index).cloned();
    match app.sort_mode {
        crate::library::SortMode::Title => {
            app.library_results.sort_by_key(|p| {
                app.library
                    .books
                    .get(p)
                    .map(|b| b.author.to_lowercase())
            });
            app.sort_mode = crate::library::SortMode::Author;
        }
        crate::library::SortMode::Author => {
            app.library_results.sort_by(|a, b| {
                let book_a = app.library.books.get(a);
                let book_b = app.library.books.get(b);
                let s_a = book_a
                    .map(|i| i.series.to_lowercase())
                    .unwrap_or_default();
                let s_b = book_b
                    .map(|i| i.series.to_lowercase())
                    .unwrap_or_default();
                if s_a.is_empty() && !s_b.is_empty() {
                    return std::cmp::Ordering::Greater;
                }
                if !s_a.is_empty() && s_b.is_empty() {
                    return std::cmp::Ordering::Less;
                }
                if s_a == s_b {
                    let n_a = book_a.map(|i| i.series_num).unwrap_or(0);
                    let n_b = book_b.map(|i| i.series_num).unwrap_or(0);
                    n_a.cmp(&n_b)
                } else {
                    s_a.cmp(&s_b)
                }
            });
            app.sort_mode = crate::library::SortMode::Series;
        }
        crate::library::SortMode::Series => {
            app.library_results.sort_by_key(|p| {
                app.library.books.get(p).map(|b| b.title.to_lowercase())
            });
            app.sort_mode = crate::library::SortMode::Title;
        }
    }
    if let Some(path) = current_path {
        if let Some(pos) =
            app.library_results.iter().position(|p| p == &path)
        {
            app.library_index = pos;
        }
    }
}

fn handle_library_open(app: &mut App, lang: Language) -> anyhow::Result<()> {
    let query = app.search_library_query.to_lowercase();
    let filtered: Vec<PathBuf> = app
        .library_results
        .iter()
        .filter(|path| {
            if query.is_empty() {
                return true;
            }
            let info = app.library.books.get(*path);
            match app.sort_mode {
                crate::library::SortMode::Title => info
                    .map(|i| i.title.to_lowercase().contains(&query))
                    .unwrap_or_default(),
                crate::library::SortMode::Author => info
                    .map(|i| i.author.to_lowercase().contains(&query))
                    .unwrap_or_default(),
                crate::library::SortMode::Series => info
                    .map(|i| i.series.to_lowercase().contains(&query))
                    .unwrap_or_default(),
            }
        })
        .cloned()
        .collect();
    if let Some(selected_path) = filtered.get(app.library_index).cloned() {
        if let Some(old_book) = app.library.books.get_mut(&app.filename) {
            old_book.last_read_line = app.scroll;
        }
        
        let parser = crate::fb2_parser::FB2Parser::new(
            &selected_path,
            &I18n::t(lang, "unknown_title"),
            &I18n::t(lang, "unknown_author"),
        );
        app.library.books.entry(selected_path.clone()).and_modify(|entry| {
            entry.title = parser.meta.title.clone();
            entry.author = parser.meta.author.clone();
            entry.series = parser.meta.series.clone();
            entry.series_num = parser.meta.sequence_number;
        });
        
        let (p, l, t, p_map) = load_book_data(&selected_path, app.width_cache, lang);
        app.filename = selected_path.clone();
        app.parser = p;
        app.lines = l;
        app.toc = t;
        app.p_map = p_map;
        app.show_footnote = false;
        app.current_footnote_text = String::new();
        app.current_footnote_scroll = 0;
        app.search_query.clear();
        app.search_results.clear();
        app.is_searching = false;
        app.show_info = false;
        app.show_toc = false;
        app.show_help = false;
        app.scroll = app
            .library
            .books
            .get(&app.filename)
            .map(|b| b.last_read_line)
            .unwrap_or(0);
        app.library.last_opened_book = Some(selected_path);
        app.state = AppState::Reader;
        app.library.save();
    }
    Ok(())
}

fn handle_settings_enter(
    app: &mut App,
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    lang: Language,
) -> anyhow::Result<()> {
    match app.config_index {
        0 => {
            app.state = AppState::InputPath;
            app.input_buffer.clear();
        }
        1 => {
            app.is_searching = false;
            app.state = AppState::Scanning;
            terminal.draw(|f| {
                let block = ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan));
                f.render_widget(block, f.size());
                let area = crate::ui::centered_rect(40, 15, f.size());
                f.render_widget(ratatui::widgets::Clear, area);
                let scan_msg = I18n::t(lang, "scanning_title") + "\n      ***      ";
                f.render_widget(
                    ratatui::widgets::Paragraph::new(scan_msg)
                        .alignment(ratatui::layout::Alignment::Center)
                        .block(
                            ratatui::widgets::Block::default()
                                .borders(ratatui::widgets::Borders::ALL)
                                .border_type(ratatui::widgets::BorderType::Double)
                                .style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)),
                        ),
                    area,
                );
            })?;
            app.library.scan();
            app.state = AppState::Config;
            app.library.save();
        }
        2 => {
            app.library.books.clear();
            app.library.save();
        }
        3 => {
            app.library.save();
        }
        4 => {
            app.state = AppState::InputUrl;
            app.input_buffer.clear();
        }
        5 => {
            app.library.language = match app.library.language {
                Language::Ru => Language::En,
                Language::En => Language::Ru,
            };
            app.library.save();
            let (p, l, t, p_map) = load_book_data(&app.filename, app.width_cache, app.library.language);
            app.parser = p;
            app.lines = l;
            app.toc = t;
            app.p_map = p_map;
        }
        6 => {
            app.library.popup_border_color = if app.library.popup_border_color == Color::White {
                app.library.theme_color
            } else {
                Color::White
            };
            app.library.save();
        }
        7 => {
            app.state = AppState::Reader;
        }
        _ => {}
    }
    Ok(())
}

fn handle_input_path_enter(app: &mut App, lang: Language) {
    let mut trimmed_path = app.input_buffer.trim().to_string();
    if trimmed_path.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            trimmed_path =
                trimmed_path.replacen('~', &home.to_string_lossy(), 1);
        }
    }
    if !trimmed_path.is_empty() {
        let new_path = std::path::PathBuf::from(&trimmed_path);
        if new_path.exists() && new_path.is_dir() {
            app.library.scan_paths = vec![new_path];
            app.state = AppState::Config;
            app.input_buffer.clear();
        } else {
            app.input_buffer = I18n::t(lang, "input_path_error");
        }
    }
}

fn handle_input_url_enter(app: &mut App, lang: Language) -> anyhow::Result<()> {
    let url = app.input_buffer.trim().to_string();
    if !url.is_empty() {
        match download_book(&url, &mut app.library, lang) {
            Ok(path) => {
                let (p, l, t, p_map) = load_book_data(&path, app.width_cache, lang);
                app.filename = path.clone();
                app.parser = p;
                app.lines = l;
                app.toc = t;
                app.p_map = p_map;
                app.scroll = 0;
                app.library.last_opened_book = Some(path);
                app.state = AppState::Reader;
                app.input_buffer.clear();
                app.library.save();
            }
            Err(e) => {
                app.input_buffer = format!("Ошибка: {}", e);
            }
        }
    }
    Ok(())
}

fn handle_footnote_open(app: &mut App, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()> {
    let size = terminal.size()?;
    let terminal_height = size.height as usize;
    let visible_height = terminal_height.saturating_sub(3);
    let bottom_scroll = (app.scroll + visible_height).min(app.lines.len());

    let mut found_note_id = None;
    for line_idx in app.scroll..bottom_scroll {
        if let Some(line) = app.lines.get(line_idx) {
            if let Some(start) = line.find("^f:[") {
                if let Some(end) = line[start..].find(']') {
                    let num_str = &line[start+4..start+end];
                    if let Ok(num) = num_str.parse::<usize>() {
                        for (_p_idx, note_id) in &app.parser.footnotes_locations {
                            let note_num = app.parser.footnotes_locations
                                .iter()
                                .position(|(_, id)| id == note_id)
                                .map(|i| i + 1)
                                .unwrap_or(0);
                            if note_num == num {
                                found_note_id = Some(note_id.clone());
                                break;
                            }
                        }
                        if found_note_id.is_some() {
                            break;
                        }
                    }
                }
            }
        }
    }

    if let Some(id) = found_note_id {
        if let Some(text) = app.parser.notes.get(&id) {
            app.current_footnote_text = text.clone();
            app.current_footnote_scroll = 0;
            app.show_footnote = true;
        }
    }
    Ok(())
}
