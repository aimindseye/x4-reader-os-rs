extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use super::model::{AppAction, AppScreen, AppShell, BrowserEntry};

#[derive(Debug, Clone, Default)]
pub struct BrowserState;

impl BrowserState {
    pub const fn new() -> Self {
        Self
    }

    pub fn seed_placeholder_entries(&mut self, shell: &mut AppShell) {
        if !shell.browser_entries().is_empty() {
            return;
        }

        let entries: Vec<BrowserEntry> = vec![
            BrowserEntry::parent(),
            BrowserEntry::directory("books"),
            BrowserEntry::directory("documents"),
            BrowserEntry::file("example.epub"),
            BrowserEntry::file("welcome.txt"),
        ];

        shell.set_browser_entries(entries);
        shell.set_browser_selected(0);
    }

    pub fn handle_action(&mut self, shell: &mut AppShell, action: AppAction) -> Option<AppScreen> {
        match action {
            AppAction::Up => {
                let next = shell.browser_selected().saturating_sub(1);
                shell.set_browser_selected(next);
                None
            }
            AppAction::Down => {
                let len = shell.browser_entries().len();
                if len > 0 {
                    let next = (shell.browser_selected() + 1).min(len - 1);
                    shell.set_browser_selected(next);
                }
                None
            }
            AppAction::Select => match shell.selected_browser_entry() {
                Some(entry) if entry.name == ".." => {
                    shell.set_screen(AppScreen::Home);
                    Some(AppScreen::Home)
                }
                Some(entry) => {
                    let name = entry.name.clone();
                    if matches!(entry.kind, super::model::BrowserEntryKind::Directory) {
                        shell.set_browser_path(name);
                        None
                    } else {
                        shell.set_screen(AppScreen::Reader);
                        Some(AppScreen::Reader)
                    }
                }
                None => None,
            },
            AppAction::Back => {
                shell.set_screen(AppScreen::Home);
                Some(AppScreen::Home)
            }
            AppAction::Left | AppAction::Right | AppAction::None => None,
        }
    }
}
