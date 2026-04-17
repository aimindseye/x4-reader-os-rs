use super::model::{AppAction, AppScreen, AppShell, HomeMenuItem};

#[derive(Debug, Clone, Default)]
pub struct HomeState;

impl HomeState {
    pub const fn new() -> Self {
        Self
    }

    pub fn selected<'a>(&self, shell: &'a AppShell) -> HomeMenuItem {
        shell.home_item()
    }

    pub fn handle_action(&mut self, shell: &mut AppShell, action: AppAction) -> Option<AppScreen> {
        match action {
            AppAction::Up => {
                let next = shell.home_selected().saturating_sub(1);
                shell.set_home_selected(next);
                None
            }
            AppAction::Down => {
                let next = (shell.home_selected() + 1).min(HomeMenuItem::ALL.len().saturating_sub(1));
                shell.set_home_selected(next);
                None
            }
            AppAction::Select => {
                let next_screen = match shell.home_item() {
                    HomeMenuItem::ContinueReading => AppScreen::Reader,
                    HomeMenuItem::FileBrowser => AppScreen::Browser,
                    HomeMenuItem::Settings => AppScreen::Home,
                };
                shell.set_screen(next_screen);
                Some(next_screen)
            }
            AppAction::Back => {
                shell.set_screen(AppScreen::Home);
                Some(AppScreen::Home)
            }
            AppAction::Left | AppAction::Right | AppAction::None => None,
        }
    }
}