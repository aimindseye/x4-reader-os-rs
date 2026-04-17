#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppScreen {
    Home,
    Browser,
    Reader,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppAction {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeMenuItem {
    ContinueReading,
    FileBrowser,
    Bookmarks,
    Settings,
    Upload,
}

impl HomeMenuItem {
    pub const fn title(self) -> &'static str {
        match self {
            HomeMenuItem::ContinueReading => "Continue",
            HomeMenuItem::FileBrowser => "Files",
            HomeMenuItem::Bookmarks => "Bookmarks",
            HomeMenuItem::Settings => "Settings",
            HomeMenuItem::Upload => "Upload",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserEntryKind {
    File,
    Directory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserEntry {
    pub name: String,
    pub kind: BrowserEntryKind,
}

impl BrowserEntry {
    pub fn file(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: BrowserEntryKind::File,
        }
    }

    pub fn directory(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: BrowserEntryKind::Directory,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReaderSession {
    pub book_path: String,
    pub current_page: u32,
    pub chapter: u16,
    pub is_epub: bool,
}

impl ReaderSession {
    pub fn new(book_path: impl Into<String>, current_page: u32, chapter: u16, is_epub: bool) -> Self {
        Self {
            book_path: book_path.into(),
            current_page,
            chapter,
            is_epub,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AppShell {
    screen: Option<AppScreen>,
    home_items: Vec<HomeMenuItem>,
    home_selected: usize,
    browser_path: String,
    browser_scroll: usize,
    browser_selected: usize,
    browser_total: usize,
    browser_entries: Vec<BrowserEntry>,
    reader_session: Option<ReaderSession>,
}

impl AppShell {
    pub fn new() -> Self {
        Self {
            screen: Some(AppScreen::Home),
            home_items: Vec::new(),
            home_selected: 0,
            browser_path: String::new(),
            browser_scroll: 0,
            browser_selected: 0,
            browser_total: 0,
            browser_entries: Vec::new(),
            reader_session: None,
        }
    }

    pub fn screen(&self) -> AppScreen {
        self.screen.unwrap_or(AppScreen::Home)
    }

    pub fn set_screen(&mut self, screen: AppScreen) {
        self.screen = Some(screen);
    }

    pub fn home_items(&self) -> &[HomeMenuItem] {
        &self.home_items
    }

    pub fn home_selected(&self) -> usize {
        self.home_selected
    }

    pub fn set_home(&mut self, items: Vec<HomeMenuItem>, selected: usize) {
        self.home_items = items;
        self.home_selected = if self.home_items.is_empty() {
            0
        } else {
            selected.min(self.home_items.len() - 1)
        };
    }

    pub fn browser_path(&self) -> &str {
        &self.browser_path
    }

    pub fn browser_scroll(&self) -> usize {
        self.browser_scroll
    }

    pub fn browser_selected(&self) -> usize {
        self.browser_selected
    }

    pub fn browser_total(&self) -> usize {
        self.browser_total
    }

    pub fn browser_entries(&self) -> &[BrowserEntry] {
        &self.browser_entries
    }

    pub fn set_browser_state(
        &mut self,
        path: impl Into<String>,
        scroll: usize,
        selected: usize,
        total: usize,
        entries: Vec<BrowserEntry>,
    ) {
        self.browser_path = path.into();
        self.browser_scroll = scroll;
        self.browser_total = total;
        self.browser_entries = entries;
        self.browser_selected = if self.browser_entries.is_empty() {
            0
        } else {
            selected.min(self.browser_entries.len() - 1)
        };
    }

    pub fn reader_session(&self) -> Option<&ReaderSession> {
        self.reader_session.as_ref()
    }

    pub fn set_reader_session(&mut self, session: ReaderSession) {
        self.reader_session = Some(session);
    }

    pub fn clear_reader_session(&mut self) {
        self.reader_session = None;
    }
}
