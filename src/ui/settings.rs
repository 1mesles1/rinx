// src/ui/settings.rs
use crate::app::App;
use crate::i18n::I18n;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::style::{Color, Style};

pub fn render_settings(f: &mut Frame, app: &App, border_style: Style, border_type: ratatui::widgets::BorderType) {
    let lang = app.library.language;
    let area = super::centered_rect(60, 35, f.area());
    f.render_widget(Clear, area);

    let lang_label = if lang == crate::i18n::Language::Ru {
        I18n::t(lang, "settings_lang_ru")
    } else {
        I18n::t(lang, "settings_lang_en")
    };

    let border_color_label = if app.library.popup_border_color == Color::White {
        I18n::t(lang, "border_color_white")
    } else {
        I18n::t(lang, "border_color_theme")
    };

    let main_border_label = match app.library.main_border {
        crate::library::BorderStyle::Plain => I18n::t(lang, "border_style_plain"),
        crate::library::BorderStyle::Double => I18n::t(lang, "border_style_double"),
        crate::library::BorderStyle::Rounded => I18n::t(lang, "border_style_rounded"),
    };

    let popup_border_label = match app.library.popup_border {
        crate::library::BorderStyle::Plain => I18n::t(lang, "border_style_plain"),
        crate::library::BorderStyle::Double => I18n::t(lang, "border_style_double"),
        crate::library::BorderStyle::Rounded => I18n::t(lang, "border_style_rounded"),
    };

    let menu_items = vec![
        I18n::t(lang, "settings_path").replace("{}", &app.library.scan_paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>().join(", ")),
        I18n::t(lang, "settings_scan").replace("{}", &app.library.books.len().to_string()),
        I18n::t(lang, "settings_clear"),
        I18n::t(lang, "settings_save"),
        I18n::t(lang, "settings_download"),
        I18n::t(lang, "settings_lang").replace("{}", &lang_label),
        I18n::t(lang, "settings_border_color").replace("{}", &border_color_label),
        I18n::t(lang, "settings_main_border").replace("{}", &main_border_label),
        I18n::t(lang, "settings_popup_border").replace("{}", &popup_border_label),
        I18n::t(lang, "settings_back"),
    ];

    let items: Vec<ListItem> = menu_items
        .iter()
        .map(|text| ListItem::new(text.as_str()))
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .title(I18n::t(lang, "settings_title"))
        .title_alignment(Alignment::Center)
        .border_style(border_style);

    let inner_area = block.inner(area);
    let visible_height = inner_area.height as usize;
    let total = items.len();
    let selected = app.config_index.min(total.saturating_sub(1));
    let mut state = ratatui::widgets::ListState::default();
    *state.offset_mut() = super::calculate_list_offset(total, selected, visible_height);
    state.select(Some(selected));

    let config_list = List::new(items)
        .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black))
        .highlight_symbol(">> ");

    f.render_stateful_widget(config_list.block(block), area, &mut state);
}

pub fn render_input_path(f: &mut Frame, app: &App, border_style: Style, border_type: ratatui::widgets::BorderType) {
    let lang = app.library.language;
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(3),
            Constraint::Percentage(45),
        ])
        .split(f.area());
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
            .border_type(border_type)
            .title(I18n::t(lang, "input_path_title"))
            .border_style(border_style),
    );
    f.render_widget(input_widget, area);
}

pub fn render_input_url(f: &mut Frame, app: &App, border_style: Style, border_type: ratatui::widgets::BorderType) {
    let lang = app.library.language;
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Length(3),
            Constraint::Percentage(45),
        ])
        .split(f.area());
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
            .border_type(border_type)
            .title(I18n::t(lang, "input_url_title"))
            .border_style(border_style),
    );
    f.render_widget(input_widget, area);
}

pub fn render_scanning(f: &mut Frame, app: &App) {
    let lang = app.library.language;
    let area = super::centered_rect(40, 10, f.area());
    f.render_widget(Clear, area);
    let scan_msg = I18n::t(lang, "scanning_msg")
        .replace("{}", &app.library.books.len().to_string());
    f.render_widget(
        Paragraph::new(scan_msg).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Double)
                .style(Style::default().fg(Color::Yellow)),
        ),
        area,
    );
}
