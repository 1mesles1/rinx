// src/main.rs

use std::path::PathBuf;
use std::time::Duration;
use std::collections::HashMap;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap, List, ListItem, ListState, Clear},
};
use ratatui::text::{Line, Span};
use ratatui::style::{Style, Color, Modifier};
use serde::{Deserialize, Serialize};
use ratatui::widgets::block::Title;

// Подключаем наши модули
mod fb2_parser;
mod layout;

// --- СТРУКТУРА ИСТОРИИ ---
#[derive(Serialize, Deserialize, Default)]
struct History {
    books: HashMap<String, usize>,
}

impl History {
    fn load() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_default()
            .join("rink_history.json");

        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                return serde_json::from_str(&content).unwrap_or_default();
            }
        }
        History::default()
    }

    fn save(&self) {
        let path = dirs::config_dir()
            .unwrap_or_default()
            .join("rink_history.json");
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, content);
        }
    }
}

// --- СОСТОЯНИЕ ПРИЛОЖЕНИЯ ---
struct App {
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
}

// Константы для ширины
const MIN_WIDTH: u16 = 30;
const MAX_WIDTH: u16 = 100;
const WIDTH_STEP: u16 = 5;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Использование: rink [ФАЙЛ]");
        return Ok(());
    }

    let filepath = PathBuf::from(&args[1]);
    
    // 1. ПАРСИНГ
    let parser = fb2_parser::FB2Parser::new(&filepath, "Неизвестно", "Неизвестный автор");

    // 2. ТЕРМИНАЛ
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 3. СОЗДАНИЕ APP
    let mut app = App {
        filename: filepath.clone(),
        lines: Vec::new(),
        scroll: 0,
        should_quit: false,
        width: 70,
        width_cache: 0,
        toc: Vec::new(),
        show_toc: false,
        toc_index: 0,
        show_info: false,
        show_help: false,
    };

// 4. ИСТОРИЯ
    let mut history = History::load();
    let path_str = filepath.to_string_lossy().to_string();
    app.scroll = *history.books.get(&path_str).unwrap_or(&0);

    // 5. ПЕРВЫЙ LAYOUT (с правильным расчетом ширины)
    let size = terminal.size()?;
    let draw_width = (size.width as u32 * app.width as u32 / 100) as u16;
    let final_w = draw_width.saturating_sub(2);
    let (lines, toc) = layout::prepare_layout(&parser.paragraphs, final_w);
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
                let (lines, toc) = layout::prepare_layout(&parser.paragraphs, current_width);
                app.lines = lines;
                app.toc = toc;
                app.width_cache = current_width; // Запоминаем, чтобы не пересчитывать каждый кадр
            }

            let block = Block::default()
                // Первый заголовок слева (Название и версия)
                .title(Title::from(format!(" rink v{} ", env!("CARGO_PKG_VERSION")))
                    .alignment(Alignment::Right))
                // Второй заголовок по центру (Название файла)
                .title(Title::from(format!(" {} ", app.filename.display()))
                    .alignment(Alignment::Center))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(Style::default().fg(Color::Cyan));

            // Собираем строки, обрабатывая заголовки (желтый цвет) и добавляя отступ слева
            let display_lines: Vec<Line> = app.lines.iter().map(|s| {
                if s.starts_with("^:") {
                    // Добавляем пробел перед заголовком
                    Line::from(Span::styled(format!(" {}", &s[2..]), Style::default().fg(Color::Yellow).bold()))
                } else {
                    // Добавляем пробел перед обычной строкой
                    Line::from(format!(" {}", s))
                }
            }).collect();

            let text_widget = Paragraph::new(display_lines)
                .block(block)
                .scroll((app.scroll as u16, 0));
            
            f.render_widget(text_widget, horizontal_chunks[1]);

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
            let encoding = &parser.encoding;

            let status_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(15),
                    Constraint::Min(0),
                    Constraint::Length(20),
                ])
                .split(chunks[1]);

            f.render_widget(
                Paragraph::new(format!(" |==| {}%", app.width)).style(Style::default().bg(Color::Blue).fg(Color::White)),
                status_chunks[0]
            );

            // 2. Центральная часть: Название книги и кодировка
            let book_title = &parser.meta.title; 
            f.render_widget(
                Paragraph::new(format!("{} [{}]", book_title, encoding))
                    .alignment(Alignment::Center)
                    .style(Style::default().bg(Color::Blue).fg(Color::White)),
                status_chunks[1]
            );

            f.render_widget(
                Paragraph::new(format!("{} {}% ", bar, progress_pct))
                    .alignment(Alignment::Right)
                    .style(Style::default().bg(Color::Blue).fg(Color::White)),
                status_chunks[2]
            );

            // --- ОГЛАВЛЕНИЕ ---
            if app.show_toc && !app.toc.is_empty() {
                let area = centered_rect(30, 75, f.size());
                f.render_widget(Clear, area);

                let items: Vec<ListItem> = app.toc.iter()
                    .map(|(title, _)| ListItem::new(title.as_str()))
                    .collect();

                let mut state = ListState::default();
                state.select(Some(app.toc_index));

                let toc_list = List::new(items)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double) // Двойная рамка
                        .title(" ОГЛАВЛЕНИЕ ")
                        .title_alignment(Alignment::Center)) // Заголовок по центру
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
                        Span::styled(" АВТОР: ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
                        Span::raw(&parser.meta.author),
                    ]),
                    Line::from(vec![
                        Span::styled(" КНИГА: ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
                        Span::raw(&parser.meta.title),
                    ]),
                ];

                if !parser.meta.series.is_empty() {
                    info_text.push(Line::from(vec![
                        Span::styled(" ЦИКЛ:  ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow)),
                        Span::raw(&parser.meta.series),
                    ]));
                }

                info_text.push(Line::from("─".repeat(area.width as usize - 2))); 
                info_text.push(Line::from(Span::styled("  АННОТАЦИЯ:", Style::default().add_modifier(Modifier::ITALIC))));
                info_text.push(Line::from(""));

                let target_w = area.width.saturating_sub(8) as usize; 
                let raw_annotation = &parser.meta.annotation;
                
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
                }

                let info_widget = Paragraph::new(info_text)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double)
                        .title(" ИНФОРМАЦИЯ О КНИГЕ ")
                        .title_alignment(Alignment::Center))
                    .wrap(Wrap { trim: false }); 
            
                f.render_widget(info_widget, area);
            } 

            // --- ОКНО ПОМОЩИ  ---
            if app.show_help {
                let area = centered_rect(30, 60, f.size());
                f.render_widget(Clear, area);

                let help_text = vec![
                    Line::from(vec![Span::styled(" УПРАВЛЕНИЕ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))]),
                    Line::from(""),
                    Line::from(" q       : Выход"),
                    Line::from(" i       : Инфо о книге"),
                    Line::from(" t       : Оглавление"),
                    Line::from(" ?       : Помощь"),
                    Line::from(""),
                    Line::from(vec![Span::styled(" НАВИГАЦИЯ", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))]),
                    Line::from(" JK/Стрелки : Вверх/Вниз"),
                    Line::from(" Space/Right: Стр. вперед"),
                    Line::from(" Left       : Стр. назад"),
                    Line::from(" +/-        : Ширина текста"),
                ];

                let help_widget = Paragraph::new(help_text)
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double)
                        .title(" КЛАВИШИ ")
                        .title_alignment(Alignment::Center));
            
                f.render_widget(help_widget, area);
            }
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        // 1. УПРАВЛЕНИЕ ОГЛАВЛЕНИЕМ (активно только когда оно открыто)
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
                        KeyCode::PageDown if app.show_toc => {
                            app.toc_index = (app.toc_index + 10).min(app.toc.len().saturating_sub(1));
                        }
                        KeyCode::PageUp if app.show_toc => {
                            app.toc_index = app.toc_index.saturating_sub(10);
                        }

                        // 2. ОБЩИЕ КОМАНДЫ (работают всегда)
                        KeyCode::Char('q') => {
                            history.books.insert(path_str.clone(), app.scroll);
                            history.save();
                            app.should_quit = true;
                        }
                        KeyCode::Char('?') => {
                            app.show_help = !app.show_help;
                            if app.show_help {
                                app.show_info = false;
                                app.show_toc = false;
                            }
                        }
                        KeyCode::Char('i') => {
                            app.show_info = !app.show_info;
                            if app.show_info { app.show_toc = false; }
                        }
                        KeyCode::Char('t') => {
                            app.show_toc = !app.show_toc;
                            if app.show_toc { app.show_info = false; }
                        }
                        KeyCode::Esc => {
                            app.show_toc = false;
                            app.show_info = false;
                            app.show_help = false;
                        }

                        // 3. НАВИГАЦИЯ ПО КНИГЕ (блокируется, если открыто любое окно)
                        KeyCode::Down | KeyCode::Char('j') if !app.show_info && !app.show_toc && !app.show_help => {
                            if app.scroll < app.lines.len().saturating_sub(1) { app.scroll += 1; }
                        }
                        KeyCode::Up | KeyCode::Char('k') if !app.show_info && !app.show_toc && !app.show_help => {
                            if app.scroll > 0 { app.scroll -= 1; }
                        }
                        KeyCode::Right | KeyCode::Char('l') | KeyCode::PageDown | KeyCode::Char(' ') 
                            if !app.show_info && !app.show_toc && !app.show_help => {
                                let visible_height = terminal.size()?.height.saturating_sub(3) as usize;
                                app.scroll = (app.scroll + visible_height).min(app.lines.len().saturating_sub(1));
                        }
                        KeyCode::Left | KeyCode::Char('h') | KeyCode::PageUp 
                            if !app.show_info && !app.show_toc && !app.show_help => {
                                let visible_height = terminal.size()?.height.saturating_sub(3) as usize;
                                app.scroll = app.scroll.saturating_sub(visible_height);
                        }
                        KeyCode::Home if !app.show_info && !app.show_toc && !app.show_help => {
                            app.scroll = 0;
                        }
                        KeyCode::End if !app.show_info && !app.show_toc && !app.show_help => {
                            let visible_height = terminal.size()?.height.saturating_sub(3) as usize;
                            app.scroll = app.lines.len().saturating_sub(visible_height);
                        }

                        // 4. НАСТРОЙКИ (работают всегда)
                        KeyCode::Char('+') | KeyCode::Char('=') | KeyCode::Char('-') => {
                            if key.code == KeyCode::Char('-') {
                                app.width = app.width.saturating_sub(5).max(30);
                            } else {
                                app.width = (app.width + 5).min(100);
                            }
                            let size = terminal.size()?;
                            let draw_width = (size.width as u32 * app.width as u32 / 100) as u16;
                            let (lines, toc) = layout::prepare_layout(&parser.paragraphs, draw_width.saturating_sub(4));
                            app.lines = lines;
                            app.toc = toc;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Восстановление терминала
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
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
