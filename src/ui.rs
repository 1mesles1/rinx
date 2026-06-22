// src/ui.rs
use crate::app::{App, AppState};
use crate::i18n::I18n;
use crate::layout;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::*;
use ratatui::widgets::block::Title;
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn render(f: &mut Frame, app: &mut App) {
    let lang = app.library.language;
    let popup_border_style = app.get_popup_border_style();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.size());

    let text_area_width = app.width;
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - text_area_width) / 2),
            Constraint::Percentage(text_area_width),
            Constraint::Percentage((100 - text_area_width) / 2),
        ])
        .split(chunks[0]);

    let current_width = horizontal_chunks[1].width.saturating_sub(4);
    if app.lines.is_empty() || app.width_cache != current_width {
        let (lines, toc, p_map) = layout::prepare_layout(&app.parser.paragraphs, current_width);
        app.lines = lines;
        app.toc = toc;
        app.p_map = p_map;
        app.width_cache = current_width;
    }

    // --- ОСНОВНОЙ БЛОК (текст книги) ---
    let block = Block::default()
        .title(
            Title::from(format!(
                " {} ",
                app.filename
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            ))
            .alignment(Alignment::Center),
        )
        .title(
            Title::from(format!(
                " {} ",
                I18n::t(lang, "app_title").replace("{}", env!("CARGO_PKG_VERSION"))
            ))
            .alignment(Alignment::Right),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().fg(app.library.theme_color));

    let view_height = chunks[0].height.saturating_sub(2) as usize;
    let display_lines: Vec<Line> = app
        .lines
        .iter()
        .skip(app.scroll)
        .take(view_height)
        .map(|s| {
            let is_header = s.starts_with("^:");
            let base_text = if is_header { &s[2..] } else { s };

            let mut spans = Vec::new();
            let mut last_pos = 0;
            let text = base_text;

            while let Some(start) = text[last_pos..].find("^f:[") {
                let abs_start = last_pos + start;
                if let Some(end) = text[abs_start..].find(']') {
                    let abs_end = abs_start + end + 1;
                    if abs_start > last_pos {
                        spans.push(Span::raw(text[last_pos..abs_start].to_string()));
                    }
                    let num_start = abs_start + 4;
                    let num_end = abs_start + end;
                    let num = &text[num_start..num_end];
                    spans.push(Span::styled(
                        format!("[{}]", num),
                        Style::default().fg(Color::Yellow).bold(),
                    ));
                    last_pos = abs_end;
                } else {
                    break;
                }
            }
            if last_pos < text.len() {
                spans.push(Span::raw(text[last_pos..].to_string()));
            }

            if spans.is_empty() {
                let style = if is_header {
                    Style::default().fg(Color::Yellow).bold()
                } else {
                    Style::default()
                };
                spans.push(Span::styled(base_text.to_string(), style));
            } else if is_header {
                for span in &mut spans {
                    span.style = Style::default().fg(Color::Yellow).bold();
                }
            }

            if !app.search_query.is_empty() && !app.search_results.is_empty() {
                let query = app.search_query.to_lowercase();
                let mut result_spans = Vec::new();
                for span in spans {
                    let text_low = span.content.to_lowercase();
                    if text_low.contains(&query) {
                        let content = span.content.clone();
                        let mut last_pos = 0;
                        for (start, part) in text_low.match_indices(&query) {
                            if start > last_pos {
                                result_spans.push(Span::raw(
                                    content[last_pos..start].to_string(),
                                ));
                            }
                            result_spans.push(Span::styled(
                                content[start..start + part.len()].to_string(),
                                Style::default().bg(Color::Red).fg(Color::White).bold(),
                            ));
                            last_pos = start + part.len();
                        }
                        if last_pos < content.len() {
                            result_spans.push(Span::raw(content[last_pos..].to_string()));
                        }
                    } else {
                        result_spans.push(span);
                    }
                }
                spans = result_spans;
            }

            let mut final_spans = vec![Span::raw(" ")];
            final_spans.extend(spans);
            Line::from(final_spans)
        })
        .collect();

    let text_widget = Paragraph::new(display_lines).block(block).scroll((0, 0));
    f.render_widget(text_widget, horizontal_chunks[1]);

    // --- НАСТРОЙКИ ---
    if matches!(app.state, AppState::Config) || matches!(app.state, AppState::InputPath) || matches!(app.state, AppState::InputUrl) {
        render_settings(f, app, popup_border_style);
    }

    // --- БИБЛИОТЕКА ---
    if let AppState::Library = app.state {
        render_library(f, app, popup_border_style);
    }

    // --- СТАТУС-БАР ---
    render_status_bar(f, app, chunks[1]);

    // --- ОГЛАВЛЕНИЕ ---
    if app.show_toc && !app.toc.is_empty() {
        render_toc(f, app, popup_border_style);
    }

    // --- ИНФОРМАЦИЯ О КНИГЕ ---
    if app.show_info {
        render_book_info(f, app, popup_border_style);
    }

    // --- ПОМОЩЬ ---
    if app.show_help {
        render_help(f, app, popup_border_style);
    }

    // --- ПОИСК ---
    if app.is_searching && !matches!(app.state, AppState::Scanning) {
        render_search(f, app, popup_border_style);
    }

    // --- СКАНИРОВАНИЕ ---
    if let AppState::Scanning = app.state {
        render_scanning(f, app);
    }

    // --- ЗАКЛАДКИ ---
    if let AppState::Bookmarks = app.state {
        render_bookmarks(f, app, popup_border_style);
    }

    // --- СНОСКА ---
    if app.show_footnote {
        render_footnote(f, app, popup_border_style);
    }

    // --- ВВОД ПУТИ ---
    if let AppState::InputPath = app.state {
        render_input_path(f, app, popup_border_style);
    }

    // --- ВВОД ССЫЛКИ ---
    if let AppState::InputUrl = app.state {
        render_input_url(f, app, popup_border_style);
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// ---- ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ ОТРИСОВКИ ----

fn render_settings(f: &mut Frame, app: &App, border_style: Style) {
    let lang = app.library.language;
    // ОКНО НАСТРОЕК — ШИРИНА: 60%  |  ВЫСОТА: 55%
    let area = centered_rect(60, 30, f.size());
    f.render_widget(Clear, area);

    let lang_label = if lang == crate::i18n::Language::Ru {
        I18n::t(lang, "settings_lang_ru")
    } else {
        I18n::t(lang, "settings_lang_en")
    };

    let border_label = if app.library.popup_border_color == Color::White {
        "Белые"
    } else {
        "Как тема"
    };

    let menu_items = vec![
        I18n::t(lang, "settings_path").replace("{}", &app.library.scan_paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>().join(", ")),
        I18n::t(lang, "settings_scan").replace("{}", &app.library.books.len().to_string()),
        I18n::t(lang, "settings_clear"),
        I18n::t(lang, "settings_save"),
        " 5. Загрузить по ссылке".to_string(),
        I18n::t(lang, "settings_lang").replace("{}", &lang_label),
        format!(" 7. Рамки: {}", border_label),
        I18n::t(lang, "settings_back"),
    ];

    let items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(i, text)| {
            let style = if i == app.config_index {
                Style::default().bg(Color::Yellow).fg(Color::Black)
            } else {
                Style::default()
            };
            ListItem::new(text.as_str()).style(style)
        })
        .collect();

    let config_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .title(I18n::t(lang, "settings_title"))
            .title_alignment(Alignment::Center)
            .border_style(border_style),
    );
    f.render_widget(config_list, area);
}

fn render_library(f: &mut Frame, app: &mut App, border_style: Style) {
    let lang = app.library.language;
    // ОКНО БИБЛИОТЕКИ — ШИРИНА: 60%  |  ВЫСОТА: 70%
    let area = centered_rect(60, 70, f.size());
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

    app.library_state.select(Some(app.library_index));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(title_text)
                .title_alignment(Alignment::Center)
                .title(
                    Title::from(format!(" {} ", selected_path))
                        .alignment(Alignment::Center)
                        .position(ratatui::widgets::block::Position::Bottom),
                )
                .border_style(border_style),
        )
        .highlight_style(Style::default().bg(Color::Green).fg(Color::Black))
        .highlight_symbol(">> ")
        .scroll_padding(10);
    f.render_stateful_widget(list, area, &mut app.library_state);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let lang = app.library.language;
    let terminal_height = f.size().height as usize;
    let visible_height = terminal_height.saturating_sub(3);
    let progress_pct = if app.lines.len() <= visible_height {
        100
    } else {
        let max_scroll = app.lines.len().saturating_sub(visible_height);
        (app.scroll * 100) / max_scroll.max(1)
    };
    let progress_pct = progress_pct.min(100);
    let bar_width = 10;
    let filled = (progress_pct * bar_width) / 100;
    let bar = format!("[{}{}]", "█".repeat(filled), " ".repeat(bar_width - filled));

    let encoding = &app.parser.encoding;
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22),
            Constraint::Min(0),
            Constraint::Length(25),
        ])
        .split(area);

    let has_bookmarks = app
        .library
        .books
        .get(&app.filename)
        .map(|b| !b.bookmarks.is_empty())
        .unwrap_or(false);
    let m_tag = if has_bookmarks {
        I18n::t(lang, "status_bookmark")
    } else {
        "    ".to_string()
    };

    f.render_widget(
        Paragraph::new(
            I18n::t(lang, "status_width").replace("{:<3}", &app.width.to_string()) + &m_tag,
        )
        .style(Style::default().bg(app.library.theme_color).fg(Color::Black)),
        status_chunks[0],
    );

    f.render_widget(
        Paragraph::new(format!("{} {}", app.parser.meta.title, I18n::t(lang, "status_encoding").replace("{}", encoding)))
            .alignment(Alignment::Center)
            .style(Style::default().bg(app.library.theme_color).fg(Color::Black)),
        status_chunks[1],
    );

    f.render_widget(
        Paragraph::new(I18n::t(lang, "status_progress").replace("{}", &bar).replace("{:>3}%", &format!("{:>3}%", progress_pct)))
            .alignment(Alignment::Right)
            .style(Style::default().bg(app.library.theme_color).fg(Color::Black)),
        status_chunks[2],
    );
}

fn render_toc(f: &mut Frame, app: &App, border_style: Style) {
    let lang = app.library.language;
    let max_toc_len = app
        .toc
        .iter()
        .map(|(t, _)| t.chars().count())
        .max()
        .unwrap_or(20);
    let desired_width = (max_toc_len + 8).max(40);
    let width_pct =
        ((desired_width as f32 / f.size().width as f32) * 100.0).min(80.0) as u16;
    // ОКНО ОГЛАВЛЕНИЯ — ШИРИНА: динамическая (width_pct)  |  ВЫСОТА: 75%
    let area = centered_rect(width_pct, 75, f.size());
    f.render_widget(Clear, area);

    let max_w = (area.width as usize).saturating_sub(6);
    let items: Vec<ListItem> = app
        .toc
        .iter()
        .map(|(title, _)| {
            let clean_title = title.trim();
            let display = if clean_title.chars().count() > max_w {
                let truncated: String =
                    clean_title.chars().take(max_w.saturating_sub(3)).collect();
                format!("{}...", truncated.trim_end())
            } else {
                clean_title.to_string()
            };
            ListItem::new(format!(" {} ", display))
        })
        .collect();

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(app.toc_index));
    let toc_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(I18n::t(lang, "toc_title"))
                .title_alignment(Alignment::Center)
                .border_style(border_style),
        )
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black))
        .highlight_symbol(">> ");
    f.render_stateful_widget(toc_list, area, &mut state);
}

fn render_book_info(f: &mut Frame, app: &App, border_style: Style) {
    let lang = app.library.language;
    // ОКНО ИНФОРМАЦИИ О КНИГЕ — ШИРИНА: 40%  |  ВЫСОТА: 70%
    let area = centered_rect(40, 70, f.size());
    f.render_widget(Clear, area);

    let mut info_text = vec![
        Line::from(vec![
            Span::styled(
                I18n::t(lang, "book_info_author"),
                Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow),
            ),
            Span::raw(&app.parser.meta.author),
        ]),
        Line::from(vec![
            Span::styled(
                I18n::t(lang, "book_info_title"),
                Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow),
            ),
            Span::raw(&app.parser.meta.title),
        ]),
    ];

    if !app.parser.meta.series.is_empty() {
        info_text.push(Line::from(vec![
            Span::styled(
                I18n::t(lang, "book_info_series"),
                Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow),
            ),
            Span::raw(&app.parser.meta.series),
        ]));
    }

    info_text.push(Line::from("─".repeat(area.width as usize - 2)));
    info_text.push(Line::from(Span::styled(
        I18n::t(lang, "book_info_annotation"),
        Style::default().add_modifier(Modifier::ITALIC),
    )));
    info_text.push(Line::from(""));

    let target_w = area.width.saturating_sub(8) as usize;
    let raw_annotation = &app.parser.meta.annotation;
    if raw_annotation.is_empty() {
        info_text.push(Line::from(format!("  {}", I18n::t(lang, "no_description"))));
    } else {
        let ann_wrapped = textwrap::fill(raw_annotation, target_w);
        let lines: Vec<_> = ann_wrapped.lines().collect();
        let len = lines.len();
        for (i, line) in lines.into_iter().enumerate() {
            let justified = if i < len - 1 {
                crate::layout::justify_line(line, target_w)
            } else {
                line.to_string()
            };
            info_text.push(Line::from(format!("  {}", justified)));
        }
    }

    let info_widget = Paragraph::new(info_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(I18n::t(lang, "book_info_title_text"))
                .title_alignment(Alignment::Center)
                .border_style(border_style),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(info_widget, area);
}

fn render_help(f: &mut Frame, app: &App, border_style: Style) {
    let lang = app.library.language;
    // ОКНО ПОМОЩИ — ШИРИНА: 30%  |  ВЫСОТА: 70%
    let area = centered_rect(30, 70, f.size());
    f.render_widget(Clear, area);

    let help_text = vec![
        I18n::t(lang, "help_controls"),
        I18n::t(lang, "help_quit"),
        I18n::t(lang, "help_settings"),
        I18n::t(lang, "help_library"),
        I18n::t(lang, "help_search_text"),
        I18n::t(lang, "help_search_next"),
        I18n::t(lang, "help_info"),
        I18n::t(lang, "help_toc"),
        I18n::t(lang, "help_theme"),
        I18n::t(lang, "help_footnote"),
        "".to_string(),
        I18n::t(lang, "help_library_title"),
        I18n::t(lang, "help_sort"),
        I18n::t(lang, "help_search_lib"),
        I18n::t(lang, "help_open"),
        "".to_string(),
        I18n::t(lang, "help_bookmarks_title"),
        I18n::t(lang, "help_bookmark_set"),
        I18n::t(lang, "help_bookmark_list"),
        I18n::t(lang, "help_bookmark_del"),
        "".to_string(),
        I18n::t(lang, "help_nav_title"),
        I18n::t(lang, "help_down"),
        I18n::t(lang, "help_page"),
        I18n::t(lang, "help_width"),
        I18n::t(lang, "help_home_end"),
    ];

    let display_help: Vec<Line> = help_text
        .iter()
        .map(|l| {
            let style = if l.starts_with(" ") || l.is_empty() {
                Style::default()
            } else {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            };
            Line::from(vec![Span::raw(" "), Span::styled(l.clone(), style)])
        })
        .collect();

    let help_widget = Paragraph::new(display_help)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(I18n::t(lang, "help_title"))
                .title_alignment(Alignment::Center)
                .border_style(border_style),
        )
        .scroll((app.library_index as u16, 0));
    f.render_widget(help_widget, area);
}

fn render_search(f: &mut Frame, app: &App, border_style: Style) {
    let lang = app.library.language;
    // ОКНО ПОИСКА — ШИРИНА: 60%  |  ВЫСОТА: 10%
    let area = centered_rect(60, 10, f.size());
    f.render_widget(Clear, area);

    let current_query = if matches!(app.state, AppState::Library) {
        &app.search_library_query
    } else {
        &app.search_query
    };

    let search_block = Paragraph::new(format!(" > {}_", current_query)).block(
        Block::default()
            .title(Span::styled(
                I18n::t(lang, "search_title"),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(border_style),
    );
    f.render_widget(search_block, area);
}

fn render_scanning(f: &mut Frame, app: &App) {
    let lang = app.library.language;
    // ОКНО СКАНИРОВАНИЯ — ШИРИНА: 40%  |  ВЫСОТА: 10%
    let area = centered_rect(40, 10, f.size());
    f.render_widget(Clear, area);
    let scan_msg = I18n::t(lang, "scanning_msg")
        .replace("{}", &app.library.books.len().to_string());
    f.render_widget(
        Paragraph::new(scan_msg).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .style(Style::default().fg(Color::Yellow)),
        ),
        area,
    );
}

fn render_bookmarks(f: &mut Frame, app: &mut App, border_style: Style) {
    let lang = app.library.language;
    // ОКНО ЗАКЛАДОК — ШИРИНА: 50%  |  ВЫСОТА: 50%
    let area = centered_rect(50, 50, f.size());
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

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(app.library_index));
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(I18n::t(lang, "bookmarks_title"))
                .title_alignment(Alignment::Center)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    f.render_stateful_widget(list, area, &mut state);
}

fn render_footnote(f: &mut Frame, app: &App, border_style: Style) {
    let lang = app.library.language;
    // ОКНО СНОСКИ — МАКСИМАЛЬНАЯ ШИРИНА: 80%  |  МАКСИМАЛЬНАЯ ВЫСОТА: 60%
    let max_width_pct = 80;
    let max_height_pct = 60;

    let raw_lines: Vec<String> = app.current_footnote_text
        .split('\n')
        .map(|s| s.to_string())
        .collect();

    let max_line_len = raw_lines.iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);

    let estimated_width = if max_line_len > 0 {
        let min_width = 40;
        let max_width = (f.size().width as usize * max_width_pct / 100).max(min_width);
        let estimated = (max_line_len + 12).min(max_width);
        estimated.max(min_width).min(max_width)
    } else {
        50
    };

    let width_pct = ((estimated_width as f32 / f.size().width as f32) * 100.0)
        .min(max_width_pct as f32)
        .max(20.0) as u16;

    let target_w = (estimated_width as usize).saturating_sub(4);

    let mut wrapped_lines: Vec<String> = Vec::new();
    for line in raw_lines {
        if line.chars().count() > target_w {
            let chars: Vec<char> = line.chars().collect();
            let mut start = 0;
            while start < chars.len() {
                let end = (start + target_w).min(chars.len());
                let part: String = chars[start..end].iter().collect();
                wrapped_lines.push(part);
                start = end;
            }
        } else {
            wrapped_lines.push(line);
        }
    }

    let line_count = wrapped_lines.len();
    let estimated_height = (line_count + 4).min(f.size().height as usize * max_height_pct / 100);
    let height_pct = ((estimated_height as f32 / f.size().height as f32) * 100.0)
        .min(max_height_pct as f32)
        .max(10.0) as u16;

    let area = centered_rect(width_pct, height_pct, f.size());
    f.render_widget(Clear, area);

    let display_lines: Vec<Line> = wrapped_lines
        .iter()
        .skip(app.current_footnote_scroll)
        .map(|l| Line::from(l.to_string()))
        .collect();

    let footnote_widget = Paragraph::new(display_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(I18n::t(lang, "footnote_title"))
                .title_alignment(Alignment::Center)
                .border_style(border_style),
        )
        .scroll((0, 0));

    f.render_widget(footnote_widget, area);
}

fn render_input_path(f: &mut Frame, app: &App, border_style: Style) {
    let lang = app.library.language;
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(3),
            Constraint::Percentage(45),
        ])
        .split(f.size());
    // ОКНО ВВОДА ПУТИ — левый отступ 21% | поле ввода 58% | правый отступ 21%
    let area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(21),
            Constraint::Percentage(58),
            Constraint::Percentage(21),
        ])
        .split(v_chunks[1])[1];
    f.render_widget(Clear, area);

    let prompt = if app.input_buffer.starts_with("ОШИБКА") || app.input_buffer.starts_with("ERROR") {
        I18n::t(lang, "input_path_error")
    } else {
        I18n::t(lang, "input_path_prompt").replace("{}", &app.input_buffer)
    };

    let input_widget = Paragraph::new(prompt).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .title(I18n::t(lang, "input_path_title"))
            .border_style(border_style),
    );
    f.render_widget(input_widget, area);
}

fn render_input_url(f: &mut Frame, app: &App, border_style: Style) {
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(3),
            Constraint::Percentage(45),
        ])
        .split(f.size());
    // ОКНО ВВОДА ССЫЛКИ — левый отступ 21% | поле ввода 58% | правый отступ 21%
    let area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(21),
            Constraint::Percentage(58),
            Constraint::Percentage(21),
        ])
        .split(v_chunks[1])[1];
    f.render_widget(Clear, area);

    let prompt = if app.input_buffer.starts_with("Ошибка") || app.input_buffer.starts_with("Error") {
        app.input_buffer.clone()
    } else {
        format!(" > {}_", app.input_buffer)
    };

    let input_widget = Paragraph::new(prompt).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .title(" ВВЕДИТЕ ССЫЛКУ НА FB2/ZIP ")
            .border_style(border_style),
    );
    f.render_widget(input_widget, area);
}
