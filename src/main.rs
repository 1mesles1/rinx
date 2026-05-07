// src/main.rs

use crate::fb2_parser::FB2Parser;
use anyhow::Result;
use crossterm::execute;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
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

// Подключаем наши модули
mod fb2_parser;
mod layout;

// --- СТРУКТУРА ИСТОРИИ ---
#[derive(Serialize, Deserialize, Default, Clone)]
struct BookEntry {
    pub title: String,
    pub author: String,
    pub series: String,
    pub series_num: i32,
    pub last_read_line: usize,
    pub bookmarks: Vec<usize>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)] // Добавил PartialEq для сравнения режимов
enum SortMode {
    Title,
    Author,
    Series,
}

#[derive(Serialize, Deserialize)]
struct Library {
    pub scan_paths: Vec<PathBuf>,
    pub last_opened_book: Option<PathBuf>,
    pub books: HashMap<PathBuf, BookEntry>,
}
impl Library {
    // загрузка из файла
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

    // Создание пустой структуры
    fn new() -> Self {
        Self {
            scan_paths: vec![std::env::current_dir().unwrap_or_default()],
            last_opened_book: None,
            books: HashMap::new(),
        }
    }

    // Сохранение
    fn save(&self) {
        let dir = dirs::config_dir().unwrap_or_default().join("rink");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("library.json");
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, content);
        }
    }

    // функция сканирования
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
                    // ИЩЕМ И FB2, И ZIP
                    if ext == "fb2" || ext == "zip" {
                        self.books.entry(p.to_path_buf()).or_insert_with(|| {
                            let parser =
                                FB2Parser::new(&p.to_path_buf(), "Неизвестно", "Неизвестный автор");
                            BookEntry {
                                title: parser.meta.title.clone(),
                                author: parser.meta.author.clone(),
                                series: parser.meta.series.clone(),
                                series_num: parser.meta.sequence_number,
                                last_read_line: 0,
                                bookmarks: Vec::new(),
                            }
                        });
                    }
                }
            }
        }
        self.save();
    }
}

enum AppState {
    Library,   // Окно выбора книг
    Reader,    // Режим чтения (то, что у нас уже есть)
    Config,    // Окно настроек (пути, сканер)
    InputPath, // Новый режим для ввода текста пути
    Scanning,
    Bookmarks,
}

struct App {
    state: AppState, // Текущий режим
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
    input_buffer: String, // НОВОЕ: для ввода путей и настроек
    search_results: Vec<usize>,
    current_search_idx: usize,
    is_searching: bool,
    show_bookmarks: bool,
    config_index: usize,
    library_results: Vec<PathBuf>, // Список путей книг для отображения
    library_index: usize,          // Курсор в списке книг
    sort_mode: SortMode,
}

// Константы для ширины
const MIN_WIDTH: u16 = 30;
const MAX_WIDTH: u16 = 100;
const WIDTH_STEP: u16 = 5;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut library = Library::load();

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

    // 2. Создаем парсер (настоящий или пустой)
    let parser = if filepath.exists() && filepath.is_file() {
        FB2Parser::new(&filepath, "Неизвестно", "Неизвестный автор")
    } else {
        FB2Parser::new(&PathBuf::new(), "", "")
    };

    // 3. ТЕРМИНАЛ
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

    // 4. СОЗДАНИЕ APP
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
        library_results: Vec::new(), // Просто создаем пустой вектор
        library_index: 0,
        sort_mode: SortMode::Title,
    };

    // ПЕРВЫЙ LAYOUT
    let size = terminal.size()?;
    let draw_width = (size.width as u32 * app.width as u32 / 100) as u16;
    let (lines, toc) = layout::prepare_layout(&app.parser.paragraphs, draw_width.saturating_sub(4));
    app.lines = lines;
    app.toc = toc;

    // Основной цикл
    while !app.should_quit {
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

            // Считаем чистую ширину внутри рамки (минус левая и правая границы)
            let current_width = horizontal_chunks[1].width.saturating_sub(4);

            // Если ширина изменилась — пересчитываем всё
            if app.lines.is_empty() || app.width_cache != current_width {
                let (lines, toc) = layout::prepare_layout(&app.parser.paragraphs, current_width);
                app.lines = lines;
                app.toc = toc;
                app.width_cache = current_width; // Запоминаем, чтобы не пересчитывать каждый кадр
            }

            let block = Block::default()
                // Центральная часть: Имя файла
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
                // Правая часть: Название и версия
                .title(
                    Title::from(format!(" rink v{} ", env!("CARGO_PKG_VERSION")))
                        .alignment(Alignment::Right),
                )
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(Color::Cyan));

            // Собираем строки, обрабатывая заголовки (желтый цвет) и добавляя отступ слева
            let display_lines: Vec<Line> = app
                .lines
                .iter()
                .enumerate()
                .map(|(_idx, s)| {
                    let is_header = s.starts_with("^:");
                    let base_text = if is_header { &s[2..] } else { s };

                    // Если строка пустая или мы ничего не ищем — просто выводим как раньше
                    if app.search_query.is_empty()
                        || !base_text
                            .to_lowercase()
                            .contains(&app.search_query.to_lowercase())
                    {
                        let style = if is_header {
                            Style::default().fg(Color::Yellow).bold()
                        } else {
                            Style::default()
                        };
                        Line::from(vec![
                            Span::raw(" "),
                            Span::styled(base_text.to_string(), style),
                        ])
                    } else {
                        // Логика подсветки совпадений
                        let mut spans = vec![Span::raw(" ")];
                        let query = app.search_query.to_lowercase();
                        let text_low = base_text.to_lowercase();
                        let mut last_pos = 0;

                        for (start, part) in text_low.match_indices(&query) {
                            // Текст ДО совпадения
                            if start > last_pos {
                                spans.push(Span::raw(&base_text[last_pos..start]));
                            }
                            // САМО совпадение (выделяем цветом)
                            let style = Style::default().bg(Color::Red).fg(Color::White).bold();
                            spans.push(Span::styled(&base_text[start..start + part.len()], style));
                            last_pos = start + part.len();
                        }

                        // Оставшийся текст ПОСЛЕ последнего совпадения
                        if last_pos < base_text.len() {
                            spans.push(Span::raw(&base_text[last_pos..]));
                        }

                        Line::from(spans)
                    }
                })
                .collect(); // Закрыли map

            let text_widget = Paragraph::new(display_lines)
                .block(block)
                .scroll((app.scroll as u16, 0));

            f.render_widget(text_widget, horizontal_chunks[1]);

            // --- ОКНО НАСТРОЕК ---
            if let AppState::Config = app.state {
                let area = centered_rect(60, 40, f.size());
                f.render_widget(Clear, area);

                let menu_items = vec![
                    format!(" 1. Путь: {:?}", app.library.scan_paths),
                    format!(" 2. Сканировать (Книг: {})", app.library.books.len()),
                    " 3. Очистить библиотеку".to_string(),
                    " 4. Сохранить настройки".to_string(),
                    " 5. Назад к чтению (Esc)".to_string(),
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
                        .title(" НАСТРОЙКИ БИБЛИОТЕКИ ")
                        .title_alignment(Alignment::Center),
                );

                f.render_widget(config_list, area);
            } // <--- Закрыли Config

            // --- ОКНО БИБЛИОТЕКИ ---
            if let AppState::Library = app.state {
                let area = centered_rect(60, 70, f.size());
                f.render_widget(Clear, area);

                // Создаем текст заголовка
                let sort_label = match app.sort_mode {
                    SortMode::Title => "Названию",
                    SortMode::Author => "Автору",
                    SortMode::Series => "Циклу",
                };

                // Создаем список элементов (items)
                let items: Vec<ListItem> = app
                    .library_results
                    .iter()
                    .map(|path| {
                        let info = app.library.books.get(path);
                        let title = info.map(|i| i.title.as_str()).unwrap_or("Без названия");
                        let author = info.map(|i| i.author.as_str()).unwrap_or("Неизвестен");
                        let series = info.map(|i| i.series.as_str()).unwrap_or("");
                        let s_num = info.map(|i| i.series_num).unwrap_or(0);

                        // Формируем строку в зависимости от режима сортировки
                        let display_string = match app.sort_mode {
                            SortMode::Author => format!(" ⎧≣⎨ {} — {}", author, title),
                            SortMode::Series => {
                                if series.is_empty() {
                                    format!(" ⎧≣⎨ {}", title)
                                } else {
                                    // Используем s_num здесь:
                                    format!(" ⎧≣⎨ ({} #{}) {}", series, s_num, title)
                                }
                            }
                            SortMode::Title => format!(" ⎧≣⎨ {} — {}", title, author),
                        };

                        ListItem::new(display_string)
                    })
                    .collect();

                // создаем состояние списка (state)
                let mut state = ListState::default();
                state.select(Some(app.library_index));

                let selected_path = app
                    .library_results
                    .get(app.library_index)
                    .map(|p| {
                        let mut p_str = p.to_string_lossy().to_string();
                        if let Some(home) = dirs::home_dir() {
                            let home_s = home.to_string_lossy().to_string();
                            if p_str.starts_with(&home_s) {
                                p_str = p_str.replacen(&home_s, "~", 1);
                            }
                        }
                        p_str
                    })
                    .unwrap_or_else(|| "...".into());

                // В самом виджете List добавляем вторую плашку заголовка вниз
                let list = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Double)
                            .title(format!(" МОЯ БИБЛИОТЕКА [Сортировка по: {}] ", sort_label))
                            .title_alignment(Alignment::Center)
                            // ПУТЬ В ФОРМАТЕ ~/ ВНИЗУ ПО ЦЕНТРУ
                            .title(
                                Title::from(format!(" {} ", selected_path))
                                    .alignment(Alignment::Center)
                                    .position(ratatui::widgets::block::Position::Bottom),
                            ),
                    )
                    .highlight_style(Style::default().bg(Color::Green).fg(Color::Black))
                    .highlight_symbol(">> ");

                // 4. Отрисовываем всё это
                f.render_stateful_widget(list, area, &mut state);
            }

            // 2. Потом блок Ввода пути (со своими расчетами внутри)
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
                        Constraint::Percentage(20),
                        Constraint::Percentage(60),
                        Constraint::Percentage(20),
                    ])
                    .split(v_chunks[1])[1];

                f.render_widget(Clear, area);
                let input_widget = Paragraph::new(format!(" > {}_", app.input_buffer)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double)
                        .title(" Введите путь для сканирования "),
                );
                f.render_widget(input_widget, area);
            }

            // --- НОВЫЙ СТАТУС-БАР ---
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
                    Constraint::Length(22), // Лево: Ширина и [M]
                    Constraint::Min(0),     // Центр: Название
                    Constraint::Length(25), // Право: Бар и %
                ])
                .split(chunks[1]);

            // ЛЕВАЯ ЧАСТЬ (Ширина и Метка закладок)
            let has_bookmarks = app
                .library
                .books
                .get(&app.filename)
                .map(|b| !b.bookmarks.is_empty())
                .unwrap_or(false);

            // Если закладки в книге есть — показываем [M], если нет — 4 пробела
            let m_tag = if has_bookmarks { " [M]" } else { "    " };

            f.render_widget(
                Paragraph::new(format!(" |==| W:{:<3}{}", app.width, m_tag))
                    .style(Style::default().bg(Color::Blue).fg(Color::White)),
                status_chunks[0],
            );

            // ЦЕНТРАЛЬНАЯ ЧАСТЬ (Название и кодировка)
            f.render_widget(
                Paragraph::new(format!("{} [{}]", app.parser.meta.title, encoding))
                    .alignment(Alignment::Center)
                    .style(Style::default().bg(Color::Blue).fg(Color::White)),
                status_chunks[1],
            );

            // ПРАВАЯ ЧАСТЬ (Прогресс без прыжков)
            // {:>3} резервирует 3 символа под цифры, чтобы текст не прыгал при 9% -> 10%
            f.render_widget(
                Paragraph::new(format!("{} {:>3}% ", bar, progress_pct))
                    .alignment(Alignment::Right)
                    .style(Style::default().bg(Color::Blue).fg(Color::White)),
                status_chunks[2],
            );

            // --- ОГЛАВЛЕНИЕ ---
            if app.show_toc && !app.toc.is_empty() {
                let area = centered_rect(30, 75, f.size());
                f.render_widget(Clear, area);

                let items: Vec<ListItem> = app
                    .toc
                    .iter()
                    .map(|(title, _)| ListItem::new(title.as_str()))
                    .collect();

                let mut state = ListState::default();
                state.select(Some(app.toc_index));

                let toc_list = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Double)
                            .title(" ОГЛАВЛЕНИЕ ")
                            .title_alignment(Alignment::Center),
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
                            " АВТОР: ",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Yellow),
                        ),
                        Span::raw(&app.parser.meta.author),
                    ]),
                    Line::from(vec![
                        Span::styled(
                            " КНИГА: ",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Yellow),
                        ),
                        Span::raw(&app.parser.meta.title),
                    ]),
                ];

                if !app.parser.meta.series.is_empty() {
                    info_text.push(Line::from(vec![
                        Span::styled(
                            " ЦИКЛ:  ",
                            Style::default()
                                .add_modifier(Modifier::BOLD)
                                .fg(Color::Yellow),
                        ),
                        Span::raw(&app.parser.meta.series),
                    ]));
                }

                info_text.push(Line::from("─".repeat(area.width as usize - 2)));
                info_text.push(Line::from(Span::styled(
                    "  АННОТАЦИЯ:",
                    Style::default().add_modifier(Modifier::ITALIC),
                )));
                info_text.push(Line::from(""));

                let target_w = area.width.saturating_sub(8) as usize;
                let raw_annotation = &app.parser.meta.annotation;

                if raw_annotation.is_empty() {
                    info_text.push(Line::from("  (нет описания)"));
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
                } // Закрыли else аннотации

                let info_widget = Paragraph::new(info_text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Double)
                            .title(" ИНФОРМАЦИЯ О КНИГЕ ")
                            .title_alignment(Alignment::Center),
                    )
                    .wrap(Wrap { trim: false });

                f.render_widget(info_widget, area);
            } // ВОТ ТУТ закрываем if app.show_info

            // --- ОКНО ПОМОЩИ ---
            if app.show_help {
                let area = centered_rect(25, 70, f.size());
                f.render_widget(Clear, area);

                let help_text = vec![
                    Line::from(vec![Span::styled(
                        "            УПРАВЛЕНИЕ",
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Yellow),
                    )]),
                    Line::from("   q       : Выход / Назад"),
                    Line::from("   o       : Настройки / Пути"),
                    Line::from("   L       : Моя Библиотека"),
                    Line::from("   /       : Поиск в тексте"),
                    Line::from("   n       : След. совпадение"),
                    Line::from("   i       : Инфо о книге"),
                    Line::from("   t       : Оглавление"),
                    Line::from("   ?       : Помощь"),
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "            ЗАКЛАДКИ",
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Yellow),
                    )]),
                    Line::from("   m       : Поставить метку"),
                    Line::from("   M       : Список закладок"),
                    Line::from("   d/Del   : Удалить (в списке)"),
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "            НАВИГАЦИЯ",
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::Yellow),
                    )]),
                    Line::from("   JK/Стрелки : Вверх/Вниз"),
                    Line::from("   Space/Right: Стр. вперед"),
                    Line::from("   Left       : Стр. назад"),
                    Line::from("   Home/End   : В начало/конец"),
                    Line::from("   +/-        : Ширина текста"),
                ];
                let help_widget = Paragraph::new(help_text).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double)
                        .title(" КЛАВИШИ ")
                        .title_alignment(Alignment::Center),
                );

                f.render_widget(help_widget, area);
            }

            // --- ОКНО ПОИСКА ---
            if app.is_searching {
                let area = centered_rect(60, 10, f.size());
                f.render_widget(Clear, area);

                let title_text = if app.search_results.is_empty() {
                    " ПОИСК [Ничего не найдено] ".to_string()
                } else {
                    format!(
                        " ПОИСК [{} / {}] ",
                        app.current_search_idx + 1,
                        app.search_results.len()
                    )
                };

                let search_block = Paragraph::new(format!(" > {}_", app.search_query)).block(
                    Block::default()
                        .title(title_text)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow))
                        .border_type(BorderType::Double),
                );

                f.render_widget(search_block, area);
            } // <--- ВОТ ТУТ закрывается поиск

            // --- ОКНО СКАНЕР --- (теперь оно само по себе)
            if let AppState::Scanning = app.state {
                let area = centered_rect(40, 10, f.size());
                f.render_widget(Clear, area);
                let scan_msg = format!(
                    "\n  [ ⎧≣⎨ ] Сканирую библиотеку...\n  Найдено книг: {}",
                    app.library.books.len()
                );
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
                                ListItem::new(format!(" Стр. {:>4} | {}...", line_idx, content))
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                // Создаем и настраиваем состояние списка
                let mut state = ListState::default();
                state.select(Some(app.library_index)); // Используем library_index как курсор

                let list = List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Double)
                            .title(" ЗАКЛАДКИ ")
                            .title_alignment(Alignment::Center),
                    )
                    .highlight_style(
                        Style::default()
                            .bg(Color::Yellow)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");

                // ВАЖНО: используем stateful виджет
                f.render_stateful_widget(list, area, &mut state);
            }
        })?; // на питоне пробелы, тут скобки ппц

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        // --- Библиотека ---
                        _ if matches!(app.state, AppState::Library) => match key.code {
                            // Выход
                            KeyCode::Esc | KeyCode::Char('q') => app.state = AppState::Reader,

                            // Удаление
                            KeyCode::Char('d') | KeyCode::Delete => {
                                if let Some(path) =
                                    app.library_results.get(app.library_index).cloned()
                                {
                                    app.library.books.remove(&path);
                                    app.library_results.remove(app.library_index);
                                    app.library_index = app
                                        .library_index
                                        .min(app.library_results.len().saturating_sub(1));
                                    app.library.save();
                                }
                            }

                            // Сортировка
                            KeyCode::Char('s') => match app.sort_mode {
                                SortMode::Title => {
                                    app.library_results.sort_by_key(|p| {
                                        app.library.books.get(p).map(|b| b.author.to_lowercase())
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
                            },

                            // Навигация
                            KeyCode::Home => app.library_index = 0,
                            KeyCode::End => {
                                app.library_index = app.library_results.len().saturating_sub(1)
                            }
                            KeyCode::PageUp | KeyCode::Left => {
                                app.library_index = app.library_index.saturating_sub(10)
                            }
                            KeyCode::PageDown | KeyCode::Right => {
                                app.library_index = (app.library_index + 10)
                                    .min(app.library_results.len().saturating_sub(1))
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.library_index = app.library_index.saturating_sub(1)
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if !app.library_results.is_empty() {
                                    app.library_index = (app.library_index + 1)
                                        .min(app.library_results.len().saturating_sub(1));
                                }
                            }

                            // Открытие книги
                            KeyCode::Enter => {
                                if let Some(selected_path) =
                                    app.library_results.get(app.library_index).cloned()
                                {
                                    if let Some(old_book) = app.library.books.get_mut(&app.filename)
                                    {
                                        old_book.last_read_line = app.scroll;
                                    }
                                    let (p, l, t) = load_book_data(&selected_path, app.width_cache);
                                    app.filename = selected_path;
                                    app.parser = p;
                                    app.lines = l;
                                    app.toc = t;
                                    app.scroll = app
                                        .library
                                        .books
                                        .get(&app.filename)
                                        .map(|b| b.last_read_line)
                                        .unwrap_or(0);
                                    app.state = AppState::Reader;
                                    app.library.save();
                                }
                            }
                            _ => {}
                        },

                        // --- 1. ОКНО НАСТРОЕК (AppState::Config) ---
                        _ if matches!(app.state, AppState::Config) => match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => app.state = AppState::Reader,
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.config_index = app.config_index.saturating_sub(1)
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if app.config_index < 4 {
                                    app.config_index += 1;
                                }
                            }
                            KeyCode::Enter => match app.config_index {
                                0 => {
                                    // ПУНКТ 1: Путь
                                    app.state = AppState::InputPath;
                                    app.input_buffer.clear(); // Очищаем БУФЕР, а не поиск
                                }
                                1 => {
                                    app.state = AppState::Scanning;
                                    terminal.draw(|f| {
                                        // Чтобы не было черного экрана, рисуем сначала основной блок (рамку)
                                        let block = Block::default()
                                            .borders(Borders::ALL)
                                            .border_type(BorderType::Rounded)
                                            .style(Style::default().fg(Color::Cyan));
                                        f.render_widget(block, f.size());

                                        // Рисуем окно сканирования чуть выше центра
                                        let area = centered_rect(40, 15, f.size());
                                        // Смещаем area чуть выше вручную, если centered_rect это позволяет,
                                        // или просто оставляем так, 15% высоты - это и так довольно узко.

                                        f.render_widget(Clear, area);

                                        // Добавляем "\n" для центровки текста по высоте внутри окна
                                        let scan_msg = "\n  СКАНИРОВАНИЕ  \n      ***      ";

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
                                }
                                2 => {
                                    app.library.books.clear();
                                    app.library.save();
                                }
                                3 => app.library.save(),
                                4 => app.state = AppState::Reader,
                                _ => {}
                            },
                            _ => {}
                        },

                        // --- ВВОД ПУТИ (AppState::InputPath) ---
                        _ if matches!(app.state, AppState::InputPath) => match key.code {
                            KeyCode::Enter => {
                                let mut trimmed_path = app.input_buffer.trim().to_string();

                                // Расширяем тильду ~/
                                if trimmed_path.starts_with('~') {
                                    if let Some(home) = dirs::home_dir() {
                                        trimmed_path =
                                            trimmed_path.replacen('~', &home.to_string_lossy(), 1);
                                    }
                                }

                                if !trimmed_path.is_empty() {
                                    let new_path = PathBuf::from(&trimmed_path);
                                    if new_path.exists() && new_path.is_dir() {
                                        // ПУТЬ ВЕРНЫЙ: сохраняем в настройки библиотеки
                                        app.library.scan_paths = vec![new_path];
                                        app.state = AppState::Config;
                                        app.input_buffer.clear();
                                    } else {
                                        app.input_buffer = "ОШИБКА: Путь не найден!".to_string();
                                    }
                                } // я всегда буду выравнивать текст, чтобы не ебаться со скобками
                            }
                            // ВЫХОД БЕЗ СОХРАНЕНИЯ: просто возвращаемся в Config, очищая временный ввод
                            KeyCode::Esc | KeyCode::Char('q') => {
                                app.state = AppState::Config;
                                app.input_buffer.clear();
                            }
                            KeyCode::Backspace => {
                                app.input_buffer.pop();
                            }
                            KeyCode::Char(c) => {
                                if app.input_buffer.starts_with("ОШИБКА") {
                                    app.input_buffer.clear();
                                }
                                app.input_buffer.push(c);
                            }
                            _ => {}
                        },

                        // --- 3. РЕЖИМ ПОИСКА (is_searching) ---
                        _ if app.is_searching => match key.code {
                            KeyCode::Enter => app.is_searching = false,
                            KeyCode::Esc | KeyCode::Char('q') => {
                                app.is_searching = false;
                                app.search_query.clear();
                                app.search_results.clear();
                            }
                            KeyCode::Backspace => {
                                app.search_query.pop();
                                app.search_results = perform_search(&app.lines, &app.search_query);
                            }
                            KeyCode::Char(c) => {
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
                            _ => {}
                        },

                        // --- 3. УПРАВЛЕНИЕ ОГЛАВЛЕНИЕМ ---
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

                        // --- ОБЫЧНЫЙ РЕЖИМ ЧТЕНИЯ ---

                        // Закрытие окон (Помощь, Инфо, Оглавление)
                        KeyCode::Esc | KeyCode::Char('q')
                            if app.show_help || app.show_info || app.show_toc =>
                        {
                            app.show_help = false;
                            app.show_info = false;
                            app.show_toc = false;
                        }

                        // Сброс поиска
                        KeyCode::Esc | KeyCode::Char('q') if !app.search_results.is_empty() => {
                            app.search_query.clear();
                            app.search_results.clear();
                        }

                        // 4.3. Выход из программы (только из режима чтения и если нет открытых окон)
                        KeyCode::Char('q')
                            if matches!(app.state, AppState::Reader)
                                && !app.show_help
                                && !app.show_info
                                && !app.show_toc =>
                        {
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

                        // Открытие Библиотеки (L)
                        KeyCode::Char('L') if !app.is_searching => {
                            app.state = AppState::Library;
                            // Собираем пути
                            app.library_results = app.library.books.keys().cloned().collect();

                            // СОРТИРУЕМ сразу при входе (по умолчанию по названию)
                            app.library_results.sort_by_key(|p| {
                                app.library.books.get(p).map(|b| b.title.to_lowercase())
                            });
                            app.sort_mode = SortMode::Title; // Ставим режим "по названию"

                            // Ищем позицию текущей книги
                            if let Some(pos) =
                                app.library_results.iter().position(|p| p == &app.filename)
                            {
                                app.library_index = pos;
                            } else {
                                app.library_index = 0;
                            }
                        }
                        _ if matches!(app.state, AppState::Bookmarks) => match key.code {
                            // Явно закрываем окно и возвращаемся в читалку
                            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('M') => {
                                app.state = AppState::Reader;
                            }

                            KeyCode::Up | KeyCode::Char('k') => {
                                app.library_index = app.library_index.saturating_sub(1);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if let Some(book) = app.library.books.get(&app.filename) {
                                    if !book.bookmarks.is_empty() {
                                        app.library_index =
                                            (app.library_index + 1).min(book.bookmarks.len() - 1);
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if let Some(book) = app.library.books.get(&app.filename) {
                                    if let Some(&line_idx) = book.bookmarks.get(app.library_index) {
                                        app.scroll = line_idx;
                                        app.state = AppState::Reader;
                                    }
                                }
                            }
                            // Удаление закладки в самом менеджере
                            KeyCode::Char('d') | KeyCode::Delete => {
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
                                } // злоебучая скобка
                            }
                            _ => {}
                        },

                        //  Служебные окна и Поиск
                        KeyCode::Char('o') => {
                            app.state = AppState::Config;
                            app.config_index = 0;
                        }
                        KeyCode::Char('/') => {
                            app.is_searching = true;
                            app.search_query.clear();
                        }
                        KeyCode::Char('?') => {
                            app.show_help = !app.show_help;
                            app.show_info = false;
                            app.show_toc = false;
                        }
                        KeyCode::Char('i') => {
                            app.show_info = !app.show_info;
                            app.show_toc = false;
                        }
                        KeyCode::Char('t') => {
                            app.show_toc = !app.show_toc;
                            app.show_info = false;
                        }

                        KeyCode::Char('n') if !app.search_results.is_empty() => {
                            app.current_search_idx =
                                (app.current_search_idx + 1) % app.search_results.len();
                            app.scroll = app.search_results[app.current_search_idx];
                        }

                        // НАВИГАЦИЯ (Чтение)
                        KeyCode::Down | KeyCode::Char('j')
                            if !app.show_toc && !app.show_help && !app.show_info =>
                        {
                            if app.scroll < app.lines.len().saturating_sub(1) {
                                app.scroll += 1;
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k')
                            if !app.show_toc && !app.show_help && !app.show_info =>
                        {
                            if app.scroll > 0 {
                                app.scroll -= 1;
                            }
                        }
                        KeyCode::Left | KeyCode::PageUp
                            if !app.show_toc && !app.show_help && !app.show_info =>
                        {
                            let v_height = terminal.size()?.height.saturating_sub(3) as usize;
                            app.scroll = app.scroll.saturating_sub(v_height);
                        }
                        KeyCode::Right | KeyCode::PageDown | KeyCode::Char(' ')
                            if !app.show_toc && !app.show_help && !app.show_info =>
                        {
                            let v_height = terminal.size()?.height.saturating_sub(3) as usize;
                            let max_scroll = app.lines.len().saturating_sub(v_height);
                            app.scroll = (app.scroll + v_height).min(max_scroll);
                        }
                        KeyCode::Home if !app.show_toc && !app.show_help && !app.show_info => {
                            app.scroll = 0
                        }
                        KeyCode::End if !app.show_toc && !app.show_help && !app.show_info => {
                            let v_height = terminal.size()?.height.saturating_sub(3) as usize;
                            app.scroll = app.lines.len().saturating_sub(v_height);
                        }
                        // Закладки
                        KeyCode::Char('m') => {
                            if let Some(book) = app.library.books.get_mut(&app.filename) {
                                if !book.bookmarks.contains(&app.scroll) {
                                    book.bookmarks.push(app.scroll);
                                    book.bookmarks.sort();
                                    app.library.save();
                                }
                            }
                        }

                        // M (большая) — открыть список закладок этой книги
                        KeyCode::Char('M') => {
                            app.state = AppState::Bookmarks;
                        }

                        // Ширина текста
                        KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char('-') => {
                            if key.code == KeyCode::Char('-') {
                                app.width = app.width.saturating_sub(5).max(30);
                            } else {
                                app.width = (app.width + 5).min(100);
                            }
                            app.width_cache = 0;
                        }
                        _ => {}
                    } // 
                } // Конец KeyEventKind::Press
            } // Конец if let Event::Key
        } // устал её искать
    } // Конец while !app.should_quit

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ (вне main)

fn load_book_data(path: &PathBuf, width: u16) -> (FB2Parser, Vec<String>, Vec<(String, usize)>) {
    // Слово parser здесь остается как есть!
    let parser = FB2Parser::new(path, "Неизвестно", "Неизвестный автор");
    let (lines, toc) = layout::prepare_layout(&parser.paragraphs, width);
    (parser, lines, toc)
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
