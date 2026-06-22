// src/main.rs
use crate::fb2_parser::FB2Parser;
use anyhow::Result;
use crossterm::execute;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::block::Title;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

mod fb2_parser;
mod i18n;
mod layout;

use i18n::{I18n, Language};

#[derive(Serialize, Deserialize, Default, Clone)]
struct BookEntry {
    pub title: String,
    pub author: String,
    pub series: String,
    pub series_num: i32,
    pub last_read_line: usize,
    pub bookmarks: Vec<usize>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
enum SortMode {
    Title,
    Author,
    Series,
}

mod theme_color_serde {
    use ratatui::style::Color;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match color {
            Color::Cyan => "Cyan",
            Color::Green => "Green",
            Color::Magenta => "Magenta",
            Color::Yellow => "Yellow",
            Color::Red => "Red",
            Color::White => "White",
            _ => "Cyan",
        };
        s.serialize(serializer)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "Cyan" => Color::Cyan,
            "Green" => Color::Green,
            "Magenta" => Color::Magenta,
            "Yellow" => Color::Yellow,
            "Red" => Color::Red,
            "White" => Color::White,
            _ => Color::Cyan,
        })
    }
}

// НОВЫЙ МОДУЛЬ для сериализации цвета рамок
mod popup_border_color_serde {
    use ratatui::style::Color;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match color {
            Color::Cyan => "Cyan",
            Color::Green => "Green",
            Color::Magenta => "Magenta",
            Color::Yellow => "Yellow",
            Color::Red => "Red",
            Color::White => "White",
            _ => "Cyan",
        };
        s.serialize(serializer)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "Cyan" => Color::Cyan,
            "Green" => Color::Green,
            "Magenta" => Color::Magenta,
            "Yellow" => Color::Yellow,
            "Red" => Color::Red,
            "White" => Color::White,
            _ => Color::Cyan,
        })
    }
}

#[derive(Serialize, Deserialize)]
struct Library {
    pub scan_paths: Vec<PathBuf>,
    pub last_opened_book: Option<PathBuf>,
    pub books: HashMap<PathBuf, BookEntry>,
    #[serde(with = "theme_color_serde")]
    pub theme_color: Color,
    #[serde(with = "popup_border_color_serde")]
    pub popup_border_color: Color,
    pub language: Language,
}

impl Library {
    fn load() -> Self {
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
    fn new() -> Self {
        Self {
            scan_paths: vec![std::env::current_dir().unwrap_or_default()],
            last_opened_book: None,
            books: HashMap::new(),
            theme_color: Color::Cyan,
            popup_border_color: Color::White,
            language: Language::Ru,
        }
    }
    fn save(&self) {
        let dir = dirs::config_dir().unwrap_or_default().join("rink");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("library.json");
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, content);
        }
    }
    fn scan(&mut self) {
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

enum AppState {
    Library,
    Reader,
    Config,
    InputPath,
    Scanning,
    Bookmarks,
}

#[allow(dead_code)]
struct App {
    state: AppState,
    library: Library,
    parser: fb2_parser::FB2Parser,
    filename: PathBuf,
    lines: Vec<String>,
    scroll: usize,
    should_quit: bool,
    width: u16,
    width_cache: u16,
    toc_index: usize,
    show_info: bool,
    show_toc: bool,
    toc: Vec<(String, usize)>,
    show_help: bool,
    search_query: String,
    input_buffer: String,
    search_results: Vec<usize>,
    current_search_idx: usize,
    is_searching: bool,
    show_bookmarks: bool,
    config_index: usize,
    library_results: Vec<PathBuf>,
    library_index: usize,
    sort_mode: SortMode,
    search_library_query: String,
    library_state: ListState,
    library_filtered: Vec<PathBuf>,
    show_footnote: bool,
    current_footnote_scroll: usize,
    current_footnote_text: String,
    p_map: HashMap<usize, usize>,
}

#[allow(dead_code)]
const MIN_WIDTH: u16 = 30;
#[allow(dead_code)]
const MAX_WIDTH: u16 = 100;
#[allow(dead_code)]
const WIDTH_STEP: u16 = 5;

fn get_popup_border_style(library: &Library) -> Style {
    Style::default().fg(library.popup_border_color)
}

fn load_book_data(
    path: &PathBuf,
    width: u16,
    lang: Language,
) -> (
    FB2Parser,
    Vec<String>,
    Vec<(String, usize)>,
    std::collections::HashMap<usize, usize>,
) {
    let parser = FB2Parser::new(
        path,
        &I18n::t(lang, "unknown_title"),
        &I18n::t(lang, "unknown_author"),
    );
    let (lines, toc, p_map) = layout::prepare_layout(&parser.paragraphs, width);
    (parser, lines, toc, p_map)
}

fn perform_search(lines: &[String], query: &str) -> Vec<usize> {
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

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "-v" || a == "--version") {
        let version = env!("CARGO_PKG_VERSION");
        let lang = Language::Ru;
        println!("{}", I18n::t(lang, "version").replace("{}", version));
        return Ok(());
    }
    if args.iter().any(|a| a == "-h" || a == "--help") {
        let lang = Language::Ru;
        println!("{}", I18n::t(lang, "help_version"));
        return Ok(());
    }

    let mut library = Library::load();
    if library.books.is_empty() {
        library.scan();
    }
    let lang = library.language;

    let (filepath, start_state) = if args.len() > 1 {
        (PathBuf::from(&args[1]), AppState::Reader)
    } else if let Some(ref last_path) = library.last_opened_book {
        if last_path.exists() {
            (last_path.clone(), AppState::Reader)
        } else {
            (PathBuf::new(), AppState::Config)
        }
    } else {
        (PathBuf::new(), AppState::Config)
    };

    let parser = if filepath.exists() && filepath.is_file() {
        FB2Parser::new(
            &filepath,
            &I18n::t(lang, "unknown_title"),
            &I18n::t(lang, "unknown_author"),
        )
    } else {
        FB2Parser::new(&PathBuf::new(), "", "")
    };

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let current_scroll = library
        .books
        .get(&filepath)
        .map(|b| b.last_read_line)
        .unwrap_or(0);

    let mut app = App {
        state: start_state,
        library,
        parser,
        filename: filepath,
        lines: Vec::new(),
        scroll: current_scroll,
        should_quit: false,
        width: 70,
        width_cache: 0,
        toc: Vec::new(),
        show_toc: false,
        toc_index: 0,
        show_info: false,
        show_help: false,
        search_query: String::new(),
        input_buffer: String::new(),
        search_results: Vec::new(),
        current_search_idx: 0,
        is_searching: false,
        show_bookmarks: false,
        config_index: 0,
        library_results: Vec::new(),
        library_index: 0,
        sort_mode: SortMode::Title,
        search_library_query: String::new(),
        library_state: ListState::default(),
        library_filtered: Vec::new(),
        show_footnote: false,
        current_footnote_scroll: 0,
        current_footnote_text: String::new(),
        p_map: HashMap::new(),
    };

    let size = terminal.size()?;
    let draw_width = (size.width as u32 * app.width as u32 / 100) as u16;
    let (lines, toc, p_map) =
        layout::prepare_layout(&app.parser.paragraphs, draw_width.saturating_sub(4));
    app.lines = lines;
    app.toc = toc;
    app.p_map = p_map;

    let _tick_rate = std::time::Duration::from_millis(30);

    while !app.should_quit {
        let lang = app.library.language;
        let popup_border_style = get_popup_border_style(&app.library);

        terminal.draw(|f| {
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
                let (lines, toc, p_map) =
                    layout::prepare_layout(&app.parser.paragraphs, current_width);
                app.lines = lines;
                app.toc = toc;
                app.p_map = p_map;
                app.width_cache = current_width;
            }

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
            if matches!(app.state, AppState::Config) || matches!(app.state, AppState::InputPath) {
                let area = centered_rect(50, 30, f.size());
                f.render_widget(Clear, area);

                let lang_label = if lang == Language::Ru {
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
                    I18n::t(lang, "settings_lang").replace("{}", &lang_label),
                    format!(" 6. Рамки: {}", border_label),
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
                        .border_style(popup_border_style),
                );
                f.render_widget(config_list, area);
            }

            // --- БИБЛИОТЕКА ---
            if let AppState::Library = app.state {
                let area = centered_rect(60, 70, f.size());
                f.render_widget(Clear, area);

                let sort_label = match app.sort_mode {
                    SortMode::Title => I18n::t(lang, "library_sort_title"),
                    SortMode::Author => I18n::t(lang, "library_sort_author"),
                    SortMode::Series => I18n::t(lang, "library_sort_series"),
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
                            SortMode::Title => info
                                .map(|i| i.title.to_lowercase().contains(&query))
                                .unwrap_or_default(),
                            SortMode::Author => info
                                .map(|i| i.author.to_lowercase().contains(&query))
                                .unwrap_or_default(),
                            SortMode::Series => info
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
                            SortMode::Author => format!(" {} — {}", author, title),
                            SortMode::Series => {
                                if series.is_empty() {
                                    format!(" {}", title)
                                } else {
                                    format!(" ({}, #{}) {}", series, s_num, title)
                                }
                            }
                            SortMode::Title => format!(" {} — {}", title, author),
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
                            .border_style(popup_border_style),
                    )
                    .highlight_style(Style::default().bg(Color::Green).fg(Color::Black))
                    .highlight_symbol(">> ")
                    .scroll_padding(10);
                f.render_stateful_widget(list, area, &mut app.library_state);
            }

            // --- СТАТУС-БАР ---
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
                .split(chunks[1]);

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

            // --- ОГЛАВЛЕНИЕ ---
            if app.show_toc && !app.toc.is_empty() {
                let max_toc_len = app
                    .toc
                    .iter()
                    .map(|(t, _)| t.chars().count())
                    .max()
                    .unwrap_or(20);
                let desired_width = (max_toc_len + 8).max(40);
                let width_pct =
                    ((desired_width as f32 / f.size().width as f32) * 100.0).min(80.0) as u16;
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

                let mut state = ListState::default();
                state.select(Some(app.toc_index));
                let toc_list = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Double)
                            .title(I18n::t(lang, "toc_title"))
                            .title_alignment(Alignment::Center)
                            .border_style(popup_border_style),
                    )
                    .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black))
                    .highlight_symbol(">> ");
                f.render_stateful_widget(toc_list, area, &mut state);
            }

            // --- ИНФОРМАЦИЯ О КНИГЕ ---
            if app.show_info {
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
                            layout::justify_line(line, target_w)
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
                            .border_style(popup_border_style),
                    )
                    .wrap(Wrap { trim: false });
                f.render_widget(info_widget, area);
            }

            // --- ПОМОЩЬ ---
            if app.show_help {
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
                            .border_style(popup_border_style),
                    )
                    .scroll((app.library_index as u16, 0));
                f.render_widget(help_widget, area);
            }

            // --- ПОИСК ---
            if app.is_searching && !matches!(app.state, AppState::Scanning) {
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
                        .border_style(popup_border_style),
                );
                f.render_widget(search_block, area);
            }

            // --- СКАНИРОВАНИЕ ---
            if let AppState::Scanning = app.state {
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

            // --- ЗАКЛАДКИ ---
            if let AppState::Bookmarks = app.state {
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

                let mut state = ListState::default();
                state.select(Some(app.library_index));
                let list = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Double)
                            .title(I18n::t(lang, "bookmarks_title"))
                            .title_alignment(Alignment::Center)
                            .border_style(popup_border_style),
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

            // --- СНОСКА ---
            if app.show_footnote {
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
                    .max(30.0) as u16;

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
                    .max(20.0) as u16;

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
                            .border_style(popup_border_style),
                    )
                    .scroll((0, 0));

                f.render_widget(footnote_widget, area);
            }

            // --- ВВОД ПУТИ (поверх всех окон) ---
            if let AppState::InputPath = app.state {
                let v_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(45),
                        Constraint::Length(3),
                        Constraint::Percentage(45),
                    ])
                    .split(f.size());
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
                        .border_style(popup_border_style),
                );
                f.render_widget(input_widget, area);
            }
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let lang = app.library.language;

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
                        continue;
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
                            let current_path = app.library_results.get(app.library_index).cloned();
                            match app.sort_mode {
                                SortMode::Title => {
                                    app.library_results.sort_by_key(|p| {
                                        app.library
                                            .books
                                            .get(p)
                                            .map(|b| b.author.to_lowercase())
                                    });
                                    app.sort_mode = SortMode::Author;
                                }
                                SortMode::Author => {
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
                                    app.sort_mode = SortMode::Series;
                                }
                                SortMode::Series => {
                                    app.library_results.sort_by_key(|p| {
                                        app.library.books.get(p).map(|b| b.title.to_lowercase())
                                    });
                                    app.sort_mode = SortMode::Title;
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
                                        SortMode::Title => info
                                            .map(|i| i.title.to_lowercase().contains(&query))
                                            .unwrap_or_default(),
                                        SortMode::Author => info
                                            .map(|i| i.author.to_lowercase().contains(&query))
                                            .unwrap_or_default(),
                                        SortMode::Series => info
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
                                
                                let parser = FB2Parser::new(
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
                        }

                        // --- НАСТРОЙКИ ---
                        KeyCode::Esc | KeyCode::Char('q') if matches!(app.state, AppState::Config) => {
                            app.state = AppState::Reader;
                        }
                        KeyCode::Up | KeyCode::Char('k') if matches!(app.state, AppState::Config) => {
                            app.config_index = app.config_index.saturating_sub(1);
                        }
                        KeyCode::Down | KeyCode::Char('j') if matches!(app.state, AppState::Config) => {
                            if app.config_index < 6 {
                                app.config_index += 1;
                            }
                        }
                        KeyCode::Enter if matches!(app.state, AppState::Config) => match app.config_index {
                            0 => {
                                app.state = AppState::InputPath;
                                app.input_buffer.clear();
                            }
                            1 => {
                                app.is_searching = false;
                                app.state = AppState::Scanning;
                                terminal.draw(|f| {
                                    let block = Block::default()
                                        .borders(Borders::ALL)
                                        .border_type(BorderType::Rounded)
                                        .style(Style::default().fg(Color::Cyan));
                                    f.render_widget(block, f.size());
                                    let area = centered_rect(40, 15, f.size());
                                    f.render_widget(Clear, area);
                                    let scan_msg = I18n::t(lang, "scanning_title") + "\n      ***      ";
                                    f.render_widget(
                                        Paragraph::new(scan_msg)
                                            .alignment(Alignment::Center)
                                            .block(
                                                Block::default()
                                                    .borders(Borders::ALL)
                                                    .border_type(BorderType::Double)
                                                    .style(Style::default().fg(Color::Yellow)),
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
                                // Язык
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
                            5 => {
                                // Рамки: переключение между белыми и цветом темы
                                app.library.popup_border_color = if app.library.popup_border_color == Color::White {
                                    app.library.theme_color
                                } else {
                                    Color::White
                                };
                                app.library.save();
                            }
                            6 => {
                                app.state = AppState::Reader;
                            }
                            _ => {}
                        },

                        // --- ВВОД ПУТИ ---
                        KeyCode::Enter if matches!(app.state, AppState::InputPath) => {
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
                            app.sort_mode = SortMode::Title;
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
                            // Если рамки были "как тема", обновляем их
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
                            let terminal_height = terminal.size().unwrap().height as usize;
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
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
