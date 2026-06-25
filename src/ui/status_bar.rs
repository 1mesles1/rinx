use crate::app::App;
use crate::i18n::I18n;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use ratatui::style::{Color, Style};

pub fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
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
            format!(" {}", I18n::t(lang, "status_width").replace("{:<3}", &app.width.to_string()) + &m_tag),
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
