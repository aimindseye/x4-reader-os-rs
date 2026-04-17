extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    Home,
    Browser,
    Reader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeMenuItem {
    ContinueReading,
    FileBrowser,
    Settings,
}

impl HomeMenuItem {
    pub const ALL: [HomeMenuItem; 3] = [
        HomeMenuItem::ContinueReading,
        HomeMenuItem::FileBrowser,
        HomeMenuItem::Settings,
    ];

    pub fn title(self) -> &'static str {
        match self {
            HomeMenuItem::ContinueReading => "Continue Reading",
            HomeMenuItem::FileBrowser => "File Browser",
            HomeMenuItem::Settings => "Settings",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserEntryKind {
    File,
    Directory,
    Parent,
    Placeholder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn parent() -> Self {
        Self {
            name: "..".into(),
            kind: BrowserEntryKind::Parent,
        }
    }

    pub fn placeholder(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: BrowserEntryKind::Placeholder,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReaderSession {
    pub book_path: String,
    pub current_page: u32,
    pub total_pages: Option<u32>,
    pub last_opened_unix: Option<u64>,
}

impl ReaderSession {
    pub fn new(book_path: impl Into<String>) -> Self {
        Self {
            book_path: book_path.into(),
            current_page: 0,
            total_pages: None,
            last_opened_unix: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppShell {
    screen: AppScreen,
    home_selected: usize,
    browser_path: String,
    browser_selected: usize,
    browser_entries: Vec<BrowserEntry>,
    reader_session: Option<ReaderSession>,
}

impl Default for AppShell {
    fn default() -> Self {
        Self::new()
    }
}

impl AppShell {
    pub fn new() -> Self {
        Self {
            screen: AppScreen::Home,
            home_selected: 0,
            browser_path: "/".into(),
            browser_selected: 0,
            browser_entries: Vec::new(),
            reader_session: None,
        }
    }

    pub fn screen(&self) -> AppScreen {
        self.screen
    }

    pub fn set_screen(&mut self, screen: AppScreen) {
        self.screen = screen;
    }

    pub fn home_selected(&self) -> usize {
        self.home_selected
    }

    pub fn set_home_selected(&mut self, idx: usize) {
        self.home_selected = idx.min(HomeMenuItem::ALL.len().saturating_sub(1));
    }

    pub fn home_item(&self) -> HomeMenuItem {
        HomeMenuItem::ALL[self.home_selected]
    }

    pub fn browser_path(&self) -> &str {
        &self.browser_path
    }

    pub fn set_browser_path(&mut self, path: impl Into<String>) {
        self.browser_path = path.into();
    }

    pub fn browser_selected(&self) -> usize {
        self.browser_selected
    }

    pub fn set_browser_selected(&mut self, idx: usize) {
        let max = self.browser_entries.len().saturating_sub(1);
        self.browser_selected = idx.min(max);
    }

    pub fn browser_entries(&self) -> &[BrowserEntry] {
        &self.browser_entries
    }

    pub fn set_browser_entries(&mut self, entries: Vec<BrowserEntry>) {
        self.browser_entries = entries;
        let max = self.browser_entries.len().saturating_sub(1);
        self.browser_selected = self.browser_selected.min(max);
    }

    pub fn selected_browser_entry(&self) -> Option<&BrowserEntry> {
        self.browser_entries.get(self.browser_selected)
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