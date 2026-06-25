// src/ui/reader_popups.rs
use crate::app::App;
use crate::i18n::I18n;
use crate::layout;
use ratatui::layout::Alignment;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use textwrap;

pub fn render_toc(f: &mut Frame, app: &App, border_style: Style, border_type: ratatui::widgets::BorderType) {
    let lang = app.library.language;
    let max_toc_len = app
        .toc
        .iter()
        .map(|(t, _)| t.chars().count())
        .max()
        .unwrap_or(20);
    let desired_width = (max_toc_len + 8).max(40);
    let width_pct =
        ((desired_width as f32 / f.area().width as f32) * 100.0).min(80.0) as u16;
    let area = super::centered_rect(width_pct, 75, f.area());
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

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .title(I18n::t(lang, "toc_title"))
        .title_alignment(Alignment::Center)
        .border_style(border_style);

    let inner_area = block.inner(area);
    let visible_height = inner_area.height as usize;
    let total = items.len();
    let selected = app.toc_index.min(total.saturating_sub(1));
    let mut state = ratatui::widgets::ListState::default();
    *state.offset_mut() = super::calculate_list_offset(total, selected, visible_height);
    state.select(Some(selected));

    let toc_list = List::new(items)
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black))
        .highlight_symbol(">> ");

    f.render_stateful_widget(toc_list.block(block), area, &mut state);
}

pub fn render_book_info(f: &mut Frame, app: &App, border_style: Style, border_type: ratatui::widgets::BorderType) {
    let lang = app.library.language;
    let area = super::centered_rect(40, 70, f.area());
    f.render_widget(Clear, area);

    let mut info_text = vec![
        Line::from(vec![
            Span::styled(
                I18n::t(lang, "book_info_author"),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(&app.parser.meta.author),
        ]),
        Line::from(vec![
            Span::styled(
                I18n::t(lang, "book_info_title"),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(&app.parser.meta.title),
        ]),
    ];

    if !app.parser.meta.series.is_empty() {
        info_text.push(Line::from(vec![
            Span::styled(
                I18n::t(lang, "book_info_series"),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(&app.parser.meta.series),
        ]));
    }

    info_text.push(Line::from("─".repeat(area.width as usize - 2)));
    info_text.push(Line::from(Span::styled(
        I18n::t(lang, "book_info_annotation"),
        Style::default(),
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
                .border_type(border_type)
                .title(I18n::t(lang, "book_info_title_text"))
                .title_alignment(Alignment::Center)
                .border_style(border_style),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(info_widget, area);
}

pub fn render_help(f: &mut Frame, app: &App, border_style: Style, border_type: ratatui::widgets::BorderType) {
    let lang = app.library.language;
    let area = super::centered_rect(30, 70, f.area());
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
                Style::default().fg(Color::Yellow)
            };
            Line::from(vec![Span::raw(" "), Span::styled(l.clone(), style)])
        })
        .collect();

    let help_widget = Paragraph::new(display_help)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(border_type)
                .title(I18n::t(lang, "help_title"))
                .title_alignment(Alignment::Center)
                .border_style(border_style),
        )
        .scroll((app.library_index as u16, 0));
    f.render_widget(help_widget, area);
}

pub fn render_search(f: &mut Frame, app: &App, border_style: Style, border_type: ratatui::widgets::BorderType) {
    let lang = app.library.language;
    let area = super::centered_rect(60, 10, f.area());
    f.render_widget(Clear, area);

    let current_query = if matches!(app.state, crate::app::AppState::Library) {
        &app.search_library_query
    } else {
        &app.search_query
    };

    let search_block = Paragraph::new(format!(" > {}_", current_query)).block(
        Block::default()
            .title(Span::styled(
                I18n::t(lang, "search_title"),
                Style::default().fg(Color::Yellow),
            ))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(border_type)
            .border_style(border_style),
    );
    f.render_widget(search_block, area);
}

pub fn render_footnote(f: &mut Frame, app: &mut App, border_style: Style, border_type: ratatui::widgets::BorderType) {
    let lang = app.library.language;
    let max_width_pct = 80;
    let max_height_pct = 60;
    let min_width = 40;
    let min_height = 5;

    let raw_lines: Vec<String> = app.current_footnote_text
        .split('\n')
        .map(|s| s.to_string())
        .collect();

    let max_inner_width = (f.area().width as usize * max_width_pct / 100).max(min_width);
    let mut wrapped_lines: Vec<String> = Vec::new();
    for line in raw_lines {
        if line.is_empty() {
            wrapped_lines.push(String::new());
        } else {
            let wrapped = textwrap::fill(&line, max_inner_width);
            for wline in wrapped.lines() {
                wrapped_lines.push(wline.to_string());
            }
        }
    }

    app.footnote_wrapped_lines = wrapped_lines.clone();

    let max_line_len = wrapped_lines.iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);

    let desired_width = max_line_len + 4;
    let max_width = (f.area().width as usize * max_width_pct / 100).max(min_width);
    let final_width = desired_width.clamp(min_width, max_width);
    let width_pct = ((final_width as f32 / f.area().width as f32) * 100.0)
        .min(max_width_pct as f32)
        .max((min_width as f32 / f.area().width as f32) * 100.0) as u16;

    let line_count = wrapped_lines.len();
    let desired_height = line_count + 2;
    let max_height = (f.area().height as usize * max_height_pct / 100).max(min_height);
    let final_height = desired_height.clamp(min_height, max_height);
    let height_pct = ((final_height as f32 / f.area().height as f32) * 100.0)
        .min(max_height_pct as f32)
        .max((min_height as f32 / f.area().height as f32) * 100.0) as u16;

    let area = super::centered_rect(width_pct, height_pct, f.area());
    f.render_widget(Clear, area);

    app.footnote_visible_height = (area.height as usize).saturating_sub(2);

    let inner_width = area.width.saturating_sub(4) as usize;
    let mut final_lines: Vec<String> = Vec::new();
    let raw_lines_orig = app.current_footnote_text.split('\n').map(|s| s.to_string()).collect::<Vec<_>>();
    for line in raw_lines_orig {
        if line.is_empty() {
            final_lines.push(String::new());
        } else {
            let wrapped = textwrap::fill(&line, inner_width);
            for wline in wrapped.lines() {
                final_lines.push(wline.to_string());
            }
        }
    }

    app.footnote_wrapped_lines = final_lines.clone();

    let display_lines: Vec<Line> = final_lines
        .iter()
        .skip(app.current_footnote_scroll)
        .map(|l| Line::from(format!("  {}", l)))
        .collect();

    let footnote_widget = Paragraph::new(display_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(border_type)
                .title(I18n::t(lang, "footnote_title"))
                .title_alignment(Alignment::Center)
                .border_style(border_style),
        )
        .scroll((0, 0));

    f.render_widget(footnote_widget, area);
}
