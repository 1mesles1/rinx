// src/i18n.rs
#![allow(unused)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    Ru,
    En,
}

impl Language {
    pub fn from_str(s: &str) -> Self {
        match s {
            "En" | "en" => Language::En,
            _ => Language::Ru,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Ru => "Ru",
            Language::En => "En",
        }
    }
}

pub struct I18n;

impl I18n {
    pub fn t(lang: Language, key: &str) -> String {
        match lang {
            Language::Ru => RU_STRINGS.get(key).unwrap_or(&key).to_string(),
            Language::En => EN_STRINGS.get(key).unwrap_or(&key).to_string(),
        }
    }
}

pub const RU_STRINGS: phf::Map<&'static str, &'static str> = phf::phf_map! {
    // ====== ОСНОВНЫЕ ======
    "app_title" => "rink v{}",
    "loading" => "Загрузка...",
    "unknown_title" => "Неизвестно",
    "unknown_author" => "Неизвестный автор",
    "no_description" => "(нет описания)",

    // ====== СТАТУС-БАР ======
    "status_width" => "W:{:<3}",
    "status_bookmark" => " [M]",
    "status_encoding" => "[{}]",
    "status_progress" => "{} {:>3}% ",

    // ====== КНИГА ======
    "book_info_author" => " АВТОР: ",
    "book_info_title" => " КНИГА: ",
    "book_info_series" => " ЦИКЛ:  ",
    "book_info_annotation" => "  АННОТАЦИЯ:",
    "book_info_title_text" => " ИНФОРМАЦИЯ О КНИГЕ ",

    // ====== ОГЛАВЛЕНИЕ ======
    "toc_title" => " ОГЛАВЛЕНИЕ ",

    // ====== ПОМОЩЬ ======
    "help_title" => " КЛАВИШИ УПРАВЛЕНИЯ ",
    "help_controls" => "          УПРАВЛЕНИЕ",
    "help_quit" => "      q       : Выход / Назад",
    "help_settings" => "      o       : Настройки / Пути",
    "help_library" => "      L       : Моя Библиотека",
    "help_search_text" => "      /       : Поиск в тексте",
    "help_search_next" => "      n / N   : Поиск Вперед / Назад",
    "help_info" => "      i       : Инфо о книге",
    "help_toc" => "      t       : Оглавление",
    "help_theme" => "      c       : Сменить Тему",
    "help_footnote" => "      f       : Открыть сноску",
    "help_library_title" => "          БИБЛИОТЕКА",
    "help_sort" => "      s       : Сортировка (Автор/Цикл/Имя)",
    "help_search_lib" => "      /       : Поиск в библиотеке",
    "help_open" => "      Enter   : Открыть выбранную книгу",
    "help_bookmarks_title" => "          ЗАКЛАДКИ",
    "help_bookmark_set" => "      m       : Поставить метку",
    "help_bookmark_list" => "      M       : Список закладок",
    "help_bookmark_del" => "      d / Del : Удалить (в списке)",
    "help_nav_title" => "          НАВИГАЦИЯ",
    "help_down" => "      j / k   : Вниз / Вверх",
    "help_page" => "      Space   : Стр. вперед",
    "help_width" => "      +/-     : Ширина текста",
    "help_home_end" => "      Home/End: В начало / конец",

    // ====== ПОИСК ======
    "search_title" => " ПОИСК ",

    // ====== БИБЛИОТЕКА ======
    "library_title" => " МОЯ БИБЛИОТЕКА ",
    "library_search" => " ПОИСК ({}): {}_ ",
    "library_results" => " РЕЗУЛЬТАТЫ ({}): {} [Esc - сброс] ",
    "library_sort" => " [Сортировка по: {}] ",
    "library_sort_title" => "Названию",
    "library_sort_author" => "Автору",
    "library_sort_series" => "Циклу",
    "library_no_results" => "Без названия",
    "library_unknown_author" => "Неизвестен",

    // ====== НАСТРОЙКИ ======
    "settings_title" => " НАСТРОЙКИ ",
    "settings_path" => " 1. Путь: {}",
    "settings_scan" => " 2. Сканировать (Книг: {})",
    "settings_clear" => " 3. Очистить библиотеку",
    "settings_save" => " 4. Сохранить настройки",
    "settings_back" => " 10. Назад к чтению (Esc)",
    "settings_lang" => " 6. Язык: {}",
    "settings_download" => " 5. Загрузить по ссылке",
    "settings_main_border" => " 8. Рамка книги: {}",
    "settings_popup_border" => " 9. Рамки окон: {}",
    "settings_border_color" => " 7. Цвет рамок: {}",
    "settings_lang_ru" => "Русский",
    "settings_lang_en" => "English",
    "input_url_title" => " ВВЕДИТЕ ССЫЛКУ НА FB2/ZIP ",

    // ====== ВВОД ПУТИ ======
    "input_path_title" => " Введите путь для сканирования ",
    "input_path_error" => "ОШИБКА: Путь не найден!",
    "input_path_prompt" => " > {}_",

    // ====== СКАНИРОВАНИЕ ======
    "scanning_title" => " СКАНИРОВАНИЕ ",
    "scanning_msg" => "\n  [ ⎧≣⎨ ] Сканирую библиотеку...\n  Найдено книг: {}",

    // ====== ЗАКЛАДКИ ======
    "bookmarks_title" => " ЗАКЛАДКИ ",
    "bookmarks_item" => " Стр. {:>4} | {}...",

    // ====== СНОСКИ ======
    "footnote_title" => " СНОСКА ",

    // ====== ОГЛАВЛЕНИЕ СНОСОК ======
    "footnotes_chapter" => "Сноски",

    // ====== ВЕРСИЯ ======
    "version" => "rink {}",
    "help_version" => "? - помощь\no - настройки",
};

pub const EN_STRINGS: phf::Map<&'static str, &'static str> = phf::phf_map! {
    // ====== ОСНОВНЫЕ ======
    "app_title" => "rink v{}",
    "loading" => "Loading...",
    "unknown_title" => "Unknown",
    "unknown_author" => "Unknown author",
    "no_description" => "(no description)",

    // ====== СТАТУС-БАР ======
    "status_width" => "W:{:<3}",
    "status_bookmark" => " [M]",
    "status_encoding" => "[{}]",
    "status_progress" => "{} {:>3}% ",

    // ====== КНИГА ======
    "book_info_author" => " AUTHOR: ",
    "book_info_title" => " TITLE: ",
    "book_info_series" => " SERIES: ",
    "book_info_annotation" => "  ANNOTATION:",
    "book_info_title_text" => " BOOK INFORMATION ",

    // ====== ОГЛАВЛЕНИЕ ======
    "toc_title" => " TABLE OF CONTENTS ",

    // ====== ПОМОЩЬ ======
    "help_title" => " KEYBOARD SHORTCUTS ",
    "help_controls" => "          CONTROLS",
    "help_quit" => "      q       : Quit / Back",
    "help_settings" => "      o       : Settings / Paths",
    "help_library" => "      L       : My Library",
    "help_search_text" => "      /       : Search in text",
    "help_search_next" => "      n / N   : Search Next / Previous",
    "help_info" => "      i       : Book info",
    "help_toc" => "      t       : Table of Contents",
    "help_theme" => "      c       : Change Theme",
    "help_footnote" => "      f       : Open footnote",
    "help_library_title" => "          LIBRARY",
    "help_sort" => "      s       : Sort (Author/Series/Title)",
    "help_search_lib" => "      /       : Search in library",
    "help_open" => "      Enter   : Open selected book",
    "help_bookmarks_title" => "          BOOKMARKS",
    "help_bookmark_set" => "      m       : Set bookmark",
    "help_bookmark_list" => "      M       : Bookmarks list",
    "help_bookmark_del" => "      d / Del : Delete (in list)",
    "help_nav_title" => "          NAVIGATION",
    "help_down" => "      j / k   : Down / Up",
    "help_page" => "      Space   : Page forward",
    "help_width" => "      +/-     : Text width",
    "help_home_end" => "      Home/End: Beginning / End",

    // ====== ПОИСК ======
    "search_title" => " SEARCH ",

    // ====== БИБЛИОТЕКА ======
    "library_title" => " MY LIBRARY ",
    "library_search" => " SEARCH ({}): {}_ ",
    "library_results" => " RESULTS ({}): {} [Esc - reset] ",
    "library_sort" => " [Sorted by: {}] ",
    "library_sort_title" => "Title",
    "library_sort_author" => "Author",
    "library_sort_series" => "Series",
    "library_no_results" => "No title",
    "library_unknown_author" => "Unknown",

    // ====== НАСТРОЙКИ ======
    "settings_title" => " SETTINGS ",
    "settings_path" => " 1. Path: {}",
    "settings_scan" => " 2. Scan (Books: {})",
    "settings_clear" => " 3. Clear library",
    "settings_save" => " 4. Save settings",
    "settings_back" => " 10. Back to reading (Esc)",
    "settings_lang" => " 6. Language: {}",
    "settings_download" => " 5. Download from URL",
    "settings_main_border" => " 8. Book border: {}",
    "settings_popup_border" => " 9. Window borders: {}",
    "settings_border_color" => " 7. Border color: {}",
    "settings_lang_ru" => "Русский",
    "settings_lang_en" => "English",
    "input_url_title" => " ENTER URL FOR FB2/ZIP ",

    // ====== ВВОД ПУТИ ======
    "input_path_title" => " Enter path to scan ",
    "input_path_error" => "ERROR: Path not found!",
    "input_path_prompt" => " > {}_",

    // ====== СКАНИРОВАНИЕ ======
    "scanning_title" => " SCANNING ",
    "scanning_msg" => "\n  [ ⎧≣⎨ ] Scanning library...\n  Books found: {}",

    // ====== ЗАКЛАДКИ ======
    "bookmarks_title" => " BOOKMARKS ",
    "bookmarks_item" => " Line {:>4} | {}...",

    // ====== СНОСКИ ======
    "footnote_title" => " FOOTNOTE ",

    // ====== ОГЛАВЛЕНИЕ СНОСОК ======
    "footnotes_chapter" => "Footnotes",

    // ====== ВЕРСИЯ ======
    "version" => "rink {}",
    "help_version" => "? - help\no - settings",
};
