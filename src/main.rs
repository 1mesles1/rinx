// src/main.rs
use anyhow::Result;
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

mod app;
mod book;
mod config;
mod fb2_parser;
mod handlers;
mod i18n;
mod layout;
mod library;
mod ui;

use app::App;
use i18n::{I18n, Language};
use library::Library;

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

    let (filepath, start_state) = if args.len() > 1 {
        (std::path::PathBuf::from(&args[1]), app::AppState::Reader)
    } else if let Some(ref last_path) = library.last_opened_book {
        if last_path.exists() {
            (last_path.clone(), app::AppState::Reader)
        } else {
            (std::path::PathBuf::new(), app::AppState::Config)
        }
    } else {
        (std::path::PathBuf::new(), app::AppState::Config)
    };

    let lang = library.language;
    let parser = if filepath.exists() && filepath.is_file() {
        fb2_parser::FB2Parser::new(
            &filepath,
            &I18n::t(lang, "unknown_title"),
            &I18n::t(lang, "unknown_author"),
        )
    } else {
        fb2_parser::FB2Parser::new(&std::path::PathBuf::new(), "", "")
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

    let mut app = App::new(
        start_state,
        library,
        parser,
        filepath,
        current_scroll,
    );

    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
