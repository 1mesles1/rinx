// src/ui/library.rs
use crate::app::App;
use crate::i18n::I18n;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};
use ratatui::style::Style;
use ratatui::text::Line;  // добавлен импорт Line

pub fn render_library(f: &mut Frame, app: &mut App, border_style: Style, border_type: ratatui::widgets::BorderType) {
    let lang = app.library.language;
    let area = super::centered_rect(60, 70, f.area());
    f.render_widget(Clear, area);

    let sort_label = match app.sort_mode {
        crate::library::SortMode::Title => I18n::t(lang, "library_sort_title"),
        crate::library::SortMode::Author => I18n::t(lang, "library_sort_author"),
        crate::library::SortMode::Series => I18n::t(lang, "library_sort_series"),
    };

    let query = app.search_library_query.to_lowercase();
    let filtered_paths: Vec<std::path::PathBuf> = app
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

    if filtered_paths.is_empty() {
        app.library_index = 0;
    } else if app.library_index >= filtered_paths.len() {
        app.library_index = filtered_paths.len().saturating_sub(1);
    }

    let title_text = if app.is_searching {
        I18n::t(lang, "library_search")
            .replace("{}", &sort_label)
            .replace("{}_", &format!("{}_", app.search_library_query))
    } else if !app.search_library_query.is_empty() {
        I18n::t(lang, "library_results")
            .replace("{}", &sort_label)
            .replace("{}", &app.search_library_query)
    } else {
        I18n::t(lang, "library_title") + &I18n::t(lang, "library_sort").replace("{}", &sort_label)
    };

    let items: Vec<ListItem> = filtered_paths
        .iter()
        .map(|path| {
            let info = app.library.books.get(path);
            let title = info
                .map(|i| i.title.as_str())
                .unwrap_or("Без названия");
            let author = info
                .map(|i| i.author.as_str())
                .unwrap_or("Неизвестен");
            let series = info.map(|i| i.series.as_str()).unwrap_or("");
            let s_num = info.map(|i| i.series_num).unwrap_or(0);
            let display_string = match app.sort_mode {
                crate::library::SortMode::Author => format!(" {} — {}", author, title),
                crate::library::SortMode::Series => {
                    if series.is_empty() {
                        format!(" {}", title)
                    } else {
                        format!(" ({}, #{}) {}", series, s_num, title)
                    }
                }
                crate::library::SortMode::Title => format!(" {} — {}", title, author),
            };
            ListItem::new(display_string)
        })
        .collect();

    let selected_path = filtered_paths
        .get(app.library_index)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "...".into());

    // Основной блок — верхний заголовок уже задан через .title(title_text)
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .title(title_text)
        .title_alignment(Alignment::Center)
        .title_bottom(Line::from(format!(" {} ", selected_path)).alignment(Alignment::Center))
        .border_style(border_style);

    let inner_area = block.inner(area);
    let visible_height = inner_area.height as usize;
    let total = items.len();
    let selected = if total > 0 { app.library_index.min(total - 1) } else { 0 };
    *app.library_state.offset_mut() = super::calculate_list_offset(total, selected, visible_height);
    app.library_state.select(Some(selected));

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::Green).fg(Color::Black))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list.block(block), area, &mut app.library_state);
}

pub fn render_bookmarks(f: &mut Frame, app: &mut App, border_style: Style, border_type: ratatui::widgets::BorderType) {
    let lang = app.library.language;
    let area = super::centered_rect(50, 50, f.area());
    f.render_widget(Clear, area);

    let book = app.library.books.get(&app.filename);
    let items: Vec<ListItem> = book
        .map(|b| {
            b.bookmarks
                .iter()
                .map(|&line_idx| {
                    let content = app
                        .lines
                        .get(line_idx)
                        .map(|s| s.trim_start_matches("^:").trim())
                        .unwrap_or("...")
                        .chars()
                        .take(50)
                        .collect::<String>();
                    ListItem::new(I18n::t(lang, "bookmarks_item")
                        .replace("{:>4}", &line_idx.to_string())
                        .replace("{}", &content))
                })
                .collect()
        })
        .unwrap_or_default();

    if !items.is_empty() && app.library_index >= items.len() {
        app.library_index = items.len() - 1;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .title(I18n::t(lang, "bookmarks_title"))
        .title_alignment(Alignment::Center)
        .border_style(border_style);

    let inner_area = block.inner(area);
    let visible_height = inner_area.height as usize;
    let total = items.len();
    let selected = if total > 0 { app.library_index.min(total - 1) } else { 0 };
    let mut state = ratatui::widgets::ListState::default();
    *state.offset_mut() = super::calculate_list_offset(total, selected, visible_height);
    state.select(Some(selected));

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list.block(block), area, &mut state);
}
