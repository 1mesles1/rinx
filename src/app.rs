// src/app.rs
use crate::handlers::handle_key_event;
use crate::layout;
use crate::library::Library;
use crate::ui::render;
use crate::fb2_parser::FB2Parser;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(PartialEq)]
pub enum AppState {
    Library,
    Reader,
    Config,
    InputPath,
    InputUrl,
    Scanning,
    Bookmarks,
}

pub struct App {
    pub state: AppState,
    pub library: Library,
    pub parser: FB2Parser,
    pub filename: PathBuf,
    pub lines: Vec<String>,
    pub scroll: usize,
    pub should_quit: bool,
    pub width: u16,
    pub width_cache: u16,
    pub toc_index: usize,
    pub show_info: bool,
    pub show_toc: bool,
    pub toc: Vec<(String, usize)>,
    pub show_help: bool,
    pub search_query: String,
    pub input_buffer: String,
    pub search_results: Vec<usize>,
    pub current_search_idx: usize,
    pub is_searching: bool,
    pub config_index: usize,
    pub library_results: Vec<PathBuf>,
    pub library_index: usize,
    pub sort_mode: crate::library::SortMode,
    pub search_library_query: String,
    pub library_state: ratatui::widgets::ListState,
    pub show_footnote: bool,
    pub current_footnote_scroll: usize,
    pub current_footnote_text: String,
    pub p_map: HashMap<usize, usize>,
}

impl App {
    pub fn new(
        state: AppState,
        library: Library,
        parser: FB2Parser,
        filename: PathBuf,
        scroll: usize,
    ) -> Self {
        let width = 70;
        let mut app = Self {
            state,
            library,
            parser,
            filename,
            lines: Vec::new(),
            scroll,
            should_quit: false,
            width,
            width_cache: 0,
            toc_index: 0,
            show_info: false,
            show_toc: false,
            toc: Vec::new(),
            show_help: false,
            search_query: String::new(),
            input_buffer: String::new(),
            search_results: Vec::new(),
            current_search_idx: 0,
            is_searching: false,
            config_index: 0,
            library_results: Vec::new(),
            library_index: 0,
            sort_mode: crate::library::SortMode::Title,
            search_library_query: String::new(),
            library_state: ratatui::widgets::ListState::default(),
            show_footnote: false,
            current_footnote_scroll: 0,
            current_footnote_text: String::new(),
            p_map: HashMap::new(),
        };

        let size = ratatui::layout::Rect::new(0, 0, 80, 24);
        let draw_width = (size.width as u32 * app.width as u32 / 100) as u16;
        let (lines, toc, p_map) =
            layout::prepare_layout(&app.parser.paragraphs, draw_width.saturating_sub(4));
        app.lines = lines;
        app.toc = toc;
        app.p_map = p_map;

        app
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> anyhow::Result<()> {
        let size = terminal.size()?;
        let draw_width = (size.width as u32 * self.width as u32 / 100) as u16;
        let (lines, toc, p_map) =
            layout::prepare_layout(&self.parser.paragraphs, draw_width.saturating_sub(4));
        self.lines = lines;
        self.toc = toc;
        self.p_map = p_map;

        while !self.should_quit {
            terminal.draw(|f| {
                render(f, self);
            })?;

            if crossterm::event::poll(Duration::from_millis(50))? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    if key.kind == crossterm::event::KeyEventKind::Press {
                        handle_key_event(key, self, terminal)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_popup_border_style(&self) -> ratatui::style::Style {
        ratatui::style::Style::default().fg(self.library.popup_border_color)
    }
}
