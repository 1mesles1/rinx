// src/ui/mod.rs
use crate::app::{App, AppState};
use crate::i18n::I18n;
use crate::layout;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

mod status_bar;
mod library;
mod reader_popups;
mod settings;

// ---- Общие утилиты ----

fn border_style_to_border_type(style: crate::library::BorderStyle) -> ratatui::widgets::BorderType {
    match style {
        crate::library::BorderStyle::Plain => ratatui::widgets::BorderType::Plain,
        crate::library::BorderStyle::Double => ratatui::widgets::BorderType::Double,
        crate::library::BorderStyle::Rounded => ratatui::widgets::BorderType::Rounded,
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

pub fn calculate_list_offset(total: usize, selected: usize, height: usize) -> usize {
    if total == 0 || total <= height {
        return 0;
    }
    let half = height / 2;
    let desired = selected.saturating_sub(half);
    let max_offset = total.saturating_sub(height);
    desired.min(max_offset)
}

fn highlight_search(
    text: &str,
    query: &str,
    footnotes: &[crate::fb2_parser::FootnoteInfo],
) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    if query.is_empty() {
        let mut last_pos = 0;
        while let Some(start) = text[last_pos..].find('[') {
            let abs_start = last_pos + start;
            if let Some(end) = text[abs_start..].find(']') {
                let abs_end = abs_start + end + 1;
                let inner = &text[abs_start + 1..abs_start + end];
                if inner.chars().all(|c| c.is_ascii_digit()) {
                    if let Ok(num) = inner.parse::<usize>() {
                        let is_footnote = footnotes.iter().any(|f| f.number == num);
                        if abs_start > last_pos {
                            spans.push(Span::raw(text[last_pos..abs_start].to_string()));
                        }
                        if is_footnote {
                            spans.push(Span::styled(
                                format!("[{}]", inner),
                                Style::default().fg(Color::Yellow),
                            ));
                        } else {
                            spans.push(Span::raw(format!("[{}]", inner)));
                        }
                        last_pos = abs_end;
                        continue;
                    }
                }
            }
            break;
        }
        if last_pos < text.len() {
            spans.push(Span::raw(text[last_pos..].to_string()));
        }
        if spans.is_empty() {
            spans.push(Span::raw(text.to_string()));
        }
        return spans;
    }

    let query_lower = query.to_lowercase();
    let text_lower = text.to_lowercase();
    let mut pos = 0;

    while let Some(start) = text_lower[pos..].find(&query_lower) {
        let abs_start = pos + start;
        let abs_end = abs_start + query_lower.len();

        let before = &text[pos..abs_start];
        if !before.is_empty() {
            spans.extend(process_text_segment(before, footnotes));
        }

        let matched = &text[abs_start..abs_end];
        spans.push(Span::styled(
            matched.to_string(),
            Style::default().bg(Color::Red).fg(Color::Black),
        ));

        pos = abs_end;
    }

    let after = &text[pos..];
    if !after.is_empty() {
        spans.extend(process_text_segment(after, footnotes));
    }

    if spans.is_empty() {
        spans.push(Span::raw(text.to_string()));
    }
    spans
}

fn process_text_segment(
    segment: &str,
    footnotes: &[crate::fb2_parser::FootnoteInfo],
) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut last_pos = 0;
    while let Some(start) = segment[last_pos..].find('[') {
        let abs_start = last_pos + start;
        if let Some(end) = segment[abs_start..].find(']') {
            let abs_end = abs_start + end + 1;
            let inner = &segment[abs_start + 1..abs_start + end];
            if inner.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(num) = inner.parse::<usize>() {
                    let is_footnote = footnotes.iter().any(|f| f.number == num);
                    if abs_start > last_pos {
                        spans.push(Span::raw(segment[last_pos..abs_start].to_string()));
                    }
                    if is_footnote {
                        spans.push(Span::styled(
                            format!("[{}]", inner),
                            Style::default().fg(Color::Yellow),
                        ));
                    } else {
                        spans.push(Span::raw(format!("[{}]", inner)));
                    }
                    last_pos = abs_end;
                    continue;
                }
            }
        }
        break;
    }
    if last_pos < segment.len() {
        spans.push(Span::raw(segment[last_pos..].to_string()));
    }
    if spans.is_empty() {
        spans.push(Span::raw(segment.to_string()));
    }
    spans
}

// ---- Главный рендер ----

pub fn render(f: &mut Frame, app: &mut App) {
    let lang = app.library.language;
    let popup_border_style = app.get_popup_border_style();
    let main_border_type = border_style_to_border_type(app.library.main_border);
    let popup_border_type = border_style_to_border_type(app.library.popup_border);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

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
        let (lines, toc) = layout::prepare_layout(&app.parser.paragraphs, current_width);
        app.lines = lines;
        app.toc = toc;
        app.width_cache = current_width;
    }

    let block = Block::default()
        .title(
            Line::from(format!(
                " {} ",
                app.filename
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            ))
            .alignment(Alignment::Center),
        )
        .title(
            Line::from(format!(
                " {} ",
                I18n::t(lang, "app_title").replace("{}", env!("CARGO_PKG_VERSION"))
            ))
            .alignment(Alignment::Right),
        )
        .borders(Borders::ALL)
        .border_type(main_border_type)
        .style(Style::default().fg(app.library.theme_color));

    let view_height = chunks[0].height.saturating_sub(2) as usize;
    let query = app.search_query.clone();
    let footnotes = &app.parser.footnotes;

    let display_lines: Vec<Line> = app
        .lines
        .iter()
        .skip(app.scroll)
        .take(view_height)
        .map(|s| {
            let is_header = s.starts_with("^:");
            let text = if is_header { &s[2..] } else { s };
            let text = text.replace("^f:", "");

            if is_header {
                return Line::from(vec![
                    Span::raw(" "),
                    Span::styled(text, Style::default().fg(Color::Yellow)),
                ]);
            }

            let spans = highlight_search(&text, &query, footnotes);
            let mut final_spans = vec![Span::raw(" ")];
            final_spans.extend(spans);
            Line::from(final_spans)
        })
        .collect();

    let text_widget = Paragraph::new(display_lines).block(block).scroll((0, 0));
    f.render_widget(text_widget, horizontal_chunks[1]);

    if matches!(app.state, AppState::Config)
        || matches!(app.state, AppState::InputPath)
        || matches!(app.state, AppState::InputUrl)
    {
        settings::render_settings(f, app, popup_border_style, popup_border_type);
    }

    if let AppState::Library = app.state {
        library::render_library(f, app, popup_border_style, popup_border_type);
    }

    status_bar::render_status_bar(f, app, chunks[1]);

    if app.show_toc && !app.toc.is_empty() {
        reader_popups::render_toc(f, app, popup_border_style, popup_border_type);
    }

    if app.show_info {
        reader_popups::render_book_info(f, app, popup_border_style, popup_border_type);
    }

    if app.show_help {
        reader_popups::render_help(f, app, popup_border_style, popup_border_type);
    }

    if app.is_searching && !matches!(app.state, AppState::Scanning) {
        reader_popups::render_search(f, app, popup_border_style, popup_border_type);
    }

    if let AppState::Scanning = app.state {
        settings::render_scanning(f, app);
    }

    if let AppState::Bookmarks = app.state {
        library::render_bookmarks(f, app, popup_border_style, popup_border_type);
    }

    if app.show_footnote {
        reader_popups::render_footnote(f, app, popup_border_style, popup_border_type);
    }

    if let AppState::InputPath = app.state {
        settings::render_input_path(f, app, popup_border_style, popup_border_type);
    }

    if let AppState::InputUrl = app.state {
        settings::render_input_url(f, app, popup_border_style, popup_border_type);
    }
}
