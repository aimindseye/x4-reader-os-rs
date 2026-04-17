#![allow(dead_code)]

#[derive(Debug, Clone, Default)]
pub struct ReaderState {
    page: u32,
    chapter: u16,
}

impl ReaderState {
    pub const fn new() -> Self {
        Self { page: 0, chapter: 0 }
    }

    pub fn page(&self) -> u32 {
        self.page
    }

    pub fn chapter(&self) -> u16 {
        self.chapter
    }

    pub fn set_position(&mut self, page: u32, chapter: u16) {
        self.page = page;
        self.chapter = chapter;
    }
}
