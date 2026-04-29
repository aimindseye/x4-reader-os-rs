// launcher screen: menu, bookmarks browser

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use crate::app::HomeMenuItem;
use crate::apps::reader_state;
use crate::apps::{App, AppContext, AppId, RECENT_FILE, Transition};
use crate::board::action::{Action, ActionEvent};
use crate::board::{SCREEN_H, SCREEN_W};
use crate::drivers::strip::StripBuffer;
use crate::fonts;
use crate::kernel::KernelHandle;
use crate::ui::{
    Alignment, BitmapDynLabel, BitmapLabel, CONTENT_TOP, FULL_CONTENT_W, HEADER_W, LARGE_MARGIN,
    Region, SECTION_GAP, TITLE_Y_OFFSET,
};

const ITEM_W: u16 = 280;
const ITEM_H: u16 = 52;
const ITEM_GAP: u16 = 14;
const ITEM_STRIDE: u16 = ITEM_H + ITEM_GAP;
const ITEM_X: u16 = (SCREEN_W - ITEM_W) / 2;

const TITLE_ITEM_GAP: u16 = 20;
const RECENT_PREVIEW_GAP: u16 = 10;

const MAX_ITEMS: usize = 5;
const RECENT_BUF_LEN: usize = 160;

// bookmark list layout (matches Files app)
// bookmark-ui-v7: group same-book bookmarks under one title to avoid row clipping.
const BM_ROW_H: u16 = 52;
const BM_ROW_GAP: u16 = 4;
const BM_ROW_STRIDE: u16 = BM_ROW_H + BM_ROW_GAP;
const BM_TITLE_Y: u16 = CONTENT_TOP + TITLE_Y_OFFSET;
const BM_HEADER_LIST_GAP: u16 = SECTION_GAP;
const BM_BOOK_TITLE_GAP: u16 = 6;
const BM_MAX_TITLE_CHARS: usize = 34;
const BM_MAX_MIXED_TITLE_CHARS: usize = 16;
const BM_MAX_ROW_CHARS: usize = 42;
const BM_STATUS_W: u16 = 144;
const BM_STATUS_X: u16 = SCREEN_W - LARGE_MARGIN - BM_STATUS_W;

const CONTENT_REGION: Region = Region::new(0, CONTENT_TOP, SCREEN_W, SCREEN_H - CONTENT_TOP);

fn compute_item_regions(
    heading_line_h: u16,
    body_line_h: u16,
    has_recent: bool,
) -> [Region; MAX_ITEMS] {
    let recent_block_h = if has_recent {
        body_line_h + RECENT_PREVIEW_GAP
    } else {
        0
    };

    let item_y = CONTENT_TOP + 8 + heading_line_h + TITLE_ITEM_GAP + recent_block_h;

    [
        Region::new(ITEM_X, item_y, ITEM_W, ITEM_H),
        Region::new(ITEM_X, item_y + ITEM_STRIDE, ITEM_W, ITEM_H),
        Region::new(ITEM_X, item_y + ITEM_STRIDE * 2, ITEM_W, ITEM_H),
        Region::new(ITEM_X, item_y + ITEM_STRIDE * 3, ITEM_W, ITEM_H),
        Region::new(ITEM_X, item_y + ITEM_STRIDE * 4, ITEM_W, ITEM_H),
    ]
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum HomeState {
    Menu,
    ShowBookmarks,
}

enum MenuAction {
    Continue,
    Push(AppId),
    OpenBookmarks,
}

pub struct HomeApp {
    state: HomeState,
    selected: usize,
    ui_fonts: fonts::UiFonts,
    item_regions: [Region; MAX_ITEMS],
    item_count: usize,

    recent_book: [u8; RECENT_BUF_LEN],
    recent_book_len: usize,
    recent_record: Option<reader_state::RecentBookRecord>,
    needs_load_recent: bool,

    bm_entries: Vec<reader_state::BookmarkIndexRecord>,
    bm_count: usize,
    bm_selected: usize,
    bm_scroll: usize,
    needs_load_bookmarks: bool,
}

impl Default for HomeApp {
    fn default() -> Self {
        Self::new()
    }
}

impl HomeApp {
    pub fn new() -> Self {
        let uf = fonts::UiFonts::for_size(0);
        Self {
            state: HomeState::Menu,
            selected: 0,
            ui_fonts: uf,
            item_regions: compute_item_regions(uf.heading.line_height, uf.body.line_height, false),
            item_count: 4, // updated after load; may include Continue
            recent_book: [0u8; RECENT_BUF_LEN],
            recent_book_len: 0,
            recent_record: None,
            needs_load_recent: false,
            bm_entries: Vec::new(),
            bm_count: 0,
            bm_selected: 0,
            bm_scroll: 0,
            needs_load_bookmarks: false,
        }
    }

    pub fn set_ui_font_size(&mut self, idx: u8) {
        self.ui_fonts = fonts::UiFonts::for_size(idx);
        self.refresh_menu_layout();
    }

    fn refresh_menu_layout(&mut self) {
        self.item_regions = compute_item_regions(
            self.ui_fonts.heading.line_height,
            self.ui_fonts.body.line_height,
            self.has_recent(),
        );
    }

    // Session state accessors for RTC persistence
    #[inline]
    pub fn state_id(&self) -> u8 {
        match self.state {
            HomeState::Menu => 0,
            HomeState::ShowBookmarks => 1,
        }
    }

    #[inline]
    pub fn selected(&self) -> usize {
        self.selected
    }

    #[inline]
    pub fn bm_selected(&self) -> usize {
        self.bm_selected
    }

    #[inline]
    pub fn bm_scroll(&self) -> usize {
        self.bm_scroll
    }

    // restore home state from RTC session data
    pub fn restore_state(
        &mut self,
        state_id: u8,
        selected: usize,
        bm_selected: usize,
        bm_scroll: usize,
    ) {
        self.state = match state_id {
            1 => HomeState::ShowBookmarks,
            _ => HomeState::Menu,
        };
        self.selected = selected;
        self.bm_selected = bm_selected;
        self.bm_scroll = bm_scroll;
        if self.state == HomeState::ShowBookmarks {
            self.needs_load_bookmarks = true;
        }
        log::info!(
            "home: restore_state state={:?} selected={}",
            self.state,
            selected
        );
    }

    fn clear_recent(&mut self) {
        self.recent_book_len = 0;
        self.recent_record = None;
    }

    fn set_recent_record(&mut self, rec: reader_state::RecentBookRecord) {
        let path = rec.open_path().trim();
        if path.is_empty() {
            self.clear_recent();
            return;
        }

        let bytes = path.as_bytes();
        let n = bytes.len().min(self.recent_book.len());
        self.recent_book[..n].copy_from_slice(&bytes[..n]);
        self.recent_book_len = n;
        self.recent_record = Some(rec);
    }

    fn load_recent_record_from_state(&mut self, k: &mut KernelHandle<'_>) -> bool {
        let mut buf = [0u8; 192];
        let size = match k.read_app_subdir_chunk(
            reader_state::STATE_DIR,
            reader_state::RECENT_RECORD_FILE,
            0,
            &mut buf,
        ) {
            Ok(n) if n > 0 => n,
            _ => return false,
        };

        let text = match core::str::from_utf8(&buf[..size]) {
            Ok(s) => s.trim(),
            Err(_) => return false,
        };

        let Some(rec) = reader_state::RecentBookRecord::decode_line(text) else {
            log::warn!("phase6: ignored invalid typed recent record");
            return false;
        };

        if rec.source_path.trim().is_empty() {
            return false;
        }

        log::info!(
            "phase8: home loaded typed recent book_id={} path={}",
            rec.book_id.as_str(),
            rec.open_path()
        );
        self.set_recent_record(rec);
        true
    }

    fn load_recent_legacy(&mut self, k: &mut KernelHandle<'_>) -> bool {
        let mut buf = [0u8; RECENT_BUF_LEN];
        match k.read_app_data_start(RECENT_FILE, &mut buf) {
            Ok((_, n)) if n > 0 => {
                let text = match core::str::from_utf8(&buf[..n.min(buf.len())]) {
                    Ok(s) => s.trim(),
                    Err(_) => {
                        self.clear_recent();
                        return false;
                    }
                };

                if text.is_empty() || text.contains('|') {
                    // Phase 6: Continue must come from typed state.  Legacy
                    // fallback is accepted only as a raw path, never as a raw
                    // encoded recent record leaked into UI/routing.
                    self.clear_recent();
                    return false;
                }

                let rec = reader_state::RecentBookRecord::from_path(text);
                log::info!(
                    "phase6: upgraded legacy recent path to typed record book_id={} path={}",
                    rec.book_id.as_str(),
                    rec.source_path
                );
                self.set_recent_record(rec);
                true
            }
            _ => {
                self.clear_recent();
                false
            }
        }
    }

    pub fn load_recent(&mut self, k: &mut KernelHandle<'_>) {
        if !self.load_recent_record_from_state(k) && !self.load_recent_legacy(k) {
            self.clear_recent();
        }
        self.rebuild_item_count();
        self.refresh_menu_layout();
        self.needs_load_recent = false;
    }

    fn rebuild_item_count(&mut self) {
        self.item_count = if self.recent_record.is_some() { 5 } else { 4 };
        if self.selected >= self.item_count {
            self.selected = 0;
        }
    }

    fn has_recent(&self) -> bool {
        self.recent_record.is_some() && self.recent_book_len > 0
    }

    pub fn recent_book_bytes(&self) -> Option<&[u8]> {
        if self.has_recent() {
            Some(&self.recent_book[..self.recent_book_len])
        } else {
            None
        }
    }

    pub fn recent_book_str(&self) -> Option<&str> {
        self.recent_book_bytes()
            .and_then(|bytes| core::str::from_utf8(bytes).ok())
            .filter(|s| !s.trim().is_empty())
    }

    pub fn recent_record(&self) -> Option<reader_state::RecentBookRecord> {
        self.recent_record.clone()
    }

    pub fn recent_book_id(&self) -> Option<alloc::string::String> {
        self.recent_record().map(|rec| rec.book_id.0)
    }

    pub fn recent_source_path(&self) -> Option<alloc::string::String> {
        self.recent_record().map(|rec| rec.source_path)
    }

    fn recent_preview_text(&self) -> Option<String> {
        let rec = self.recent_record.as_ref()?;
        let title = rec.ui_title().trim();
        if title.is_empty() {
            None
        } else {
            Some(String::from(Self::basename(title)))
        }
    }

    fn basename(path: &str) -> &str {
        path.rsplit('/').next().unwrap_or(path)
    }

    pub fn shell_menu_items(&self) -> Vec<HomeMenuItem> {
        let mut items = Vec::with_capacity(self.item_count);
        if self.has_recent() {
            items.push(HomeMenuItem::ContinueReading);
        }
        items.push(HomeMenuItem::FileBrowser);
        items.push(HomeMenuItem::Bookmarks);
        items.push(HomeMenuItem::Settings);
        items.push(HomeMenuItem::Upload);
        items
    }

    fn item_label(&self, idx: usize) -> &str {
        if self.has_recent() {
            match idx {
                0 => "Continue",
                1 => "Files",
                2 => "Bookmarks",
                3 => "Settings",
                _ => "Upload",
            }
        } else {
            match idx {
                0 => "Files",
                1 => "Bookmarks",
                2 => "Settings",
                _ => "Upload",
            }
        }
    }

    fn item_action(&self, idx: usize) -> MenuAction {
        if self.has_recent() {
            match idx {
                0 => MenuAction::Continue,
                1 => MenuAction::Push(AppId::Files),
                2 => MenuAction::OpenBookmarks,
                3 => MenuAction::Push(AppId::Settings),
                _ => MenuAction::Push(AppId::Upload),
            }
        } else {
            match idx {
                0 => MenuAction::Push(AppId::Files),
                1 => MenuAction::OpenBookmarks,
                2 => MenuAction::Push(AppId::Settings),
                _ => MenuAction::Push(AppId::Upload),
            }
        }
    }

    fn move_selection(&mut self, delta: isize, ctx: &mut AppContext) {
        let count = self.item_count;
        if count == 0 {
            return;
        }
        let new = (self.selected as isize + delta).rem_euclid(count as isize) as usize;
        if new != self.selected {
            ctx.mark_dirty(self.item_regions[self.selected]);
            self.selected = new;
            ctx.mark_dirty(self.item_regions[self.selected]);
        }
    }

    fn bm_group_title(&self) -> Option<&str> {
        let first = self.bm_entries.first()?;
        let first_id = first.book_id.as_str();

        if self
            .bm_entries
            .iter()
            .all(|entry| entry.book_id.as_str() == first_id)
        {
            let title = first.display_title.trim();
            if !title.is_empty() {
                Some(title)
            } else {
                Some(Self::basename(&first.source_path))
            }
        } else {
            None
        }
    }

    fn ellipsize_ascii(s: &str, max_chars: usize) -> String {
        let trimmed = s.trim();
        let mut out = String::new();
        let mut chars = trimmed.chars();

        for _ in 0..max_chars {
            match chars.next() {
                Some(ch) => out.push(ch),
                None => return out,
            }
        }

        if chars.next().is_some() && max_chars > 3 {
            for _ in 0..3 {
                out.pop();
            }
            out.push_str("...");
        }

        out
    }

    fn bm_detail_label(entry: &reader_state::BookmarkIndexRecord) -> String {
        let detail = entry.label.trim();
        if !detail.is_empty() {
            return Self::ellipsize_ascii(detail, BM_MAX_ROW_CHARS);
        }

        let mut out = String::new();
        let _ = write!(out, "Ch {}", u32::from(entry.chapter) + 1);
        if entry.byte_offset > 0 {
            let _ = write!(out, " · Off {}", entry.byte_offset);
        }
        out
    }

    fn bm_mixed_book_label(entry: &reader_state::BookmarkIndexRecord) -> String {
        let title = if !entry.display_title.trim().is_empty() {
            entry.display_title.trim()
        } else {
            Self::basename(&entry.source_path)
        };

        let mut out = Self::ellipsize_ascii(title, BM_MAX_MIXED_TITLE_CHARS);
        out.push_str(" · ");
        out.push_str(&Self::bm_detail_label(entry));
        Self::ellipsize_ascii(&out, BM_MAX_ROW_CHARS)
    }

    fn bm_subtitle_region(&self) -> Region {
        Region::new(
            LARGE_MARGIN,
            BM_TITLE_Y + self.ui_fonts.heading.line_height + BM_BOOK_TITLE_GAP,
            FULL_CONTENT_W,
            self.ui_fonts.body.line_height,
        )
    }

    fn bm_list_y(&self) -> u16 {
        let mut y = BM_TITLE_Y + self.ui_fonts.heading.line_height + BM_HEADER_LIST_GAP;
        if self.bm_group_title().is_some() {
            y += self.ui_fonts.body.line_height + BM_BOOK_TITLE_GAP;
        }
        y
    }

    fn bm_visible_lines(&self) -> usize {
        let available = SCREEN_H.saturating_sub(self.bm_list_y());
        let rows = (available / BM_ROW_STRIDE) as usize;
        rows.max(1).min(64)
    }

    fn bm_row_region(&self, i: usize) -> Region {
        Region::new(
            LARGE_MARGIN,
            self.bm_list_y() + i as u16 * BM_ROW_STRIDE,
            FULL_CONTENT_W,
            BM_ROW_H,
        )
    }

    fn bm_list_region(&self) -> Region {
        let vis = self.bm_visible_lines();
        let subtitle_extra = if self.bm_group_title().is_some() {
            self.ui_fonts.body.line_height + BM_BOOK_TITLE_GAP
        } else {
            0
        };

        Region::new(
            LARGE_MARGIN,
            BM_TITLE_Y + self.ui_fonts.heading.line_height + BM_BOOK_TITLE_GAP,
            FULL_CONTENT_W,
            subtitle_extra + BM_ROW_STRIDE * vis as u16,
        )
    }

    fn bm_status_region(&self) -> Region {
        Region::new(
            BM_STATUS_X,
            BM_TITLE_Y,
            BM_STATUS_W,
            self.ui_fonts.heading.line_height,
        )
    }

    fn recent_preview_region(&self) -> Region {
        Region::new(
            ITEM_X,
            CONTENT_TOP + 8 + self.ui_fonts.heading.line_height + 6,
            ITEM_W,
            self.ui_fonts.body.line_height,
        )
    }
}

impl App<AppId> for HomeApp {
    fn on_enter(&mut self, ctx: &mut AppContext, _k: &mut KernelHandle<'_>) {
        ctx.clear_message();
        self.state = HomeState::Menu;
        self.selected = 0;
        self.needs_load_recent = true;
        self.needs_load_bookmarks = true;
        ctx.mark_dirty(CONTENT_REGION);
    }

    fn on_resume(&mut self, ctx: &mut AppContext, _k: &mut KernelHandle<'_>) {
        self.state = HomeState::Menu;
        self.selected = 0;
        self.needs_load_recent = true;
        self.needs_load_bookmarks = true;
        ctx.mark_dirty(CONTENT_REGION);
    }

    async fn background(&mut self, ctx: &mut AppContext, k: &mut KernelHandle<'_>) {
        if self.needs_load_recent {
            let old_count = self.item_count;
            if !self.load_recent_record_from_state(k) && !self.load_recent_legacy(k) {
                self.clear_recent();
            }
            self.rebuild_item_count();
            self.needs_load_recent = false;
            self.refresh_menu_layout();
            if self.item_count != old_count {
                ctx.request_full_redraw();
            }
        }

        if self.needs_load_bookmarks {
            let mut buf = [0u8; 4096];
            self.bm_entries.clear();

            if let Ok(n) = k.read_app_subdir_chunk(
                reader_state::STATE_DIR,
                reader_state::BOOKMARKS_INDEX_FILE,
                0,
                &mut buf,
            ) {
                if n > 0 {
                    if let Ok(payload) = core::str::from_utf8(&buf[..n]) {
                        self.bm_entries = reader_state::decode_bookmarks_index(payload);
                    }
                }
            }

            self.bm_count = self.bm_entries.len();
            log::info!(
                "bookmark-fix-v6: home loaded {} item(s) from global bookmark index",
                self.bm_count
            );
            if self.bm_count == 0 {
                self.bm_selected = 0;
                self.bm_scroll = 0;
            } else {
                if self.bm_selected >= self.bm_count {
                    self.bm_selected = self.bm_count - 1;
                }
                let vis = self.bm_visible_lines();
                if self.bm_scroll + vis <= self.bm_selected {
                    self.bm_scroll = self.bm_selected.saturating_sub(vis.saturating_sub(1));
                }
            }

            self.needs_load_bookmarks = false;
            if self.state == HomeState::ShowBookmarks {
                ctx.mark_dirty(self.bm_list_region());
                ctx.mark_dirty(self.bm_status_region());
            }
        }
    }

    fn on_event(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match self.state {
            HomeState::Menu => self.on_event_menu(event, ctx),
            HomeState::ShowBookmarks => self.on_event_bookmarks(event, ctx),
        }
    }

    fn draw(&self, strip: &mut StripBuffer) {
        match self.state {
            HomeState::Menu => self.draw_menu(strip),
            HomeState::ShowBookmarks => self.draw_bookmarks(strip),
        }
    }
}

impl HomeApp {
    fn on_event_menu(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Next) => {
                self.move_selection(1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::Prev) => {
                self.move_selection(-1, ctx);
                Transition::None
            }
            ActionEvent::Press(Action::Select) => match self.item_action(self.selected) {
                MenuAction::Continue => {
                    if let Some(rec) = self.recent_record() {
                        log::info!(
                            "phase8: continue from typed recent book_id={} path={}",
                            rec.book_id.as_str(),
                            rec.open_path()
                        );
                        ctx.set_message(rec.open_path().as_bytes());
                        Transition::Push(AppId::Reader)
                    } else {
                        log::warn!("phase6: Continue selected without typed recent record");
                        Transition::None
                    }
                }
                MenuAction::Push(app) => Transition::Push(app),
                MenuAction::OpenBookmarks => {
                    self.bm_selected = 0;
                    self.bm_scroll = 0;
                    self.needs_load_bookmarks = true;
                    self.state = HomeState::ShowBookmarks;
                    ctx.request_full_redraw();
                    Transition::None
                }
            },
            _ => Transition::None,
        }
    }

    fn on_event_bookmarks(&mut self, event: ActionEvent, ctx: &mut AppContext) -> Transition {
        match event {
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back) => {
                self.state = HomeState::Menu;
                ctx.request_full_redraw();
                Transition::None
            }

            ActionEvent::Press(Action::Next) | ActionEvent::Repeat(Action::Next) => {
                if self.bm_count > 0 {
                    let old = self.bm_selected;
                    let vis = self.bm_visible_lines();
                    if self.bm_selected + 1 < self.bm_count {
                        self.bm_selected += 1;
                        if self.bm_selected >= self.bm_scroll + vis {
                            self.bm_scroll = self.bm_selected + 1 - vis;
                            ctx.mark_dirty(self.bm_list_region());
                        } else {
                            ctx.mark_dirty(self.bm_row_region(old - self.bm_scroll));
                            ctx.mark_dirty(self.bm_row_region(self.bm_selected - self.bm_scroll));
                        }
                    } else {
                        self.bm_selected = 0;
                        self.bm_scroll = 0;
                        ctx.mark_dirty(self.bm_list_region());
                    }
                    ctx.mark_dirty(self.bm_status_region());
                }
                Transition::None
            }

            ActionEvent::Press(Action::Prev) | ActionEvent::Repeat(Action::Prev) => {
                if self.bm_count > 0 {
                    let old = self.bm_selected;
                    let vis = self.bm_visible_lines();
                    if self.bm_selected > 0 {
                        self.bm_selected -= 1;
                        if self.bm_selected < self.bm_scroll {
                            self.bm_scroll = self.bm_selected;
                            ctx.mark_dirty(self.bm_list_region());
                        } else {
                            ctx.mark_dirty(self.bm_row_region(old - self.bm_scroll));
                            ctx.mark_dirty(self.bm_row_region(self.bm_selected - self.bm_scroll));
                        }
                    } else {
                        self.bm_selected = self.bm_count - 1;
                        if self.bm_selected >= vis {
                            self.bm_scroll = self.bm_selected + 1 - vis;
                        }
                        ctx.mark_dirty(self.bm_list_region());
                    }
                    ctx.mark_dirty(self.bm_status_region());
                }
                Transition::None
            }

            ActionEvent::Press(Action::NextJump) => {
                if self.bm_count > 0 {
                    let vis = self.bm_visible_lines();
                    self.bm_selected = (self.bm_selected + vis).min(self.bm_count - 1);
                    if self.bm_selected >= self.bm_scroll + vis {
                        self.bm_scroll = self.bm_selected + 1 - vis;
                    }
                    ctx.mark_dirty(self.bm_list_region());
                    ctx.mark_dirty(self.bm_status_region());
                }
                Transition::None
            }

            ActionEvent::Press(Action::PrevJump) => {
                let vis = self.bm_visible_lines();
                self.bm_selected = self.bm_selected.saturating_sub(vis);
                if self.bm_selected < self.bm_scroll {
                    self.bm_scroll = self.bm_selected;
                }
                ctx.mark_dirty(self.bm_list_region());
                ctx.mark_dirty(self.bm_status_region());
                Transition::None
            }

            ActionEvent::Press(Action::Select) => {
                if self.bm_count > 0 && self.bm_selected < self.bm_count {
                    let slot = &self.bm_entries[self.bm_selected];
                    log::info!(
                        "bookmark-fix-v6: home bookmark selected idx={} ch={} off={} label={}",
                        self.bm_selected,
                        slot.chapter,
                        slot.byte_offset,
                        slot.display_label()
                    );
                    let jump = slot.jump_message();
                    ctx.set_message(jump.as_bytes());
                    self.state = HomeState::Menu;
                    Transition::Push(AppId::Reader)
                } else {
                    Transition::None
                }
            }

            _ => Transition::None,
        }
    }
}

impl HomeApp {
    fn draw_menu(&self, strip: &mut StripBuffer) {
        let title_region = Region::new(
            ITEM_X,
            CONTENT_TOP + 8,
            ITEM_W,
            self.ui_fonts.heading.line_height,
        );
        BitmapLabel::new(title_region, "Home", self.ui_fonts.heading)
            .alignment(Alignment::Center)
            .draw(strip)
            .unwrap();

        if let Some(recent) = self.recent_preview_text() {
            let mut preview =
                BitmapDynLabel::<48>::new(self.recent_preview_region(), self.ui_fonts.body)
                    .alignment(Alignment::Center);
            let _ = write!(preview, "{}", recent);
            preview.draw(strip).unwrap();
        }

        for i in 0..self.item_count {
            let label = self.item_label(i);
            BitmapLabel::new(self.item_regions[i], label, self.ui_fonts.body)
                .alignment(Alignment::Center)
                .inverted(i == self.selected)
                .draw(strip)
                .unwrap();
        }
    }

    fn draw_bookmarks(&self, strip: &mut StripBuffer) {
        let header_region = Region::new(
            LARGE_MARGIN,
            BM_TITLE_Y,
            HEADER_W,
            self.ui_fonts.heading.line_height,
        );
        BitmapLabel::new(header_region, "Bookmarks", self.ui_fonts.heading)
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();

        if self.bm_count > 0 {
            let mut status = BitmapDynLabel::<20>::new(self.bm_status_region(), self.ui_fonts.body)
                .alignment(Alignment::CenterRight);
            let _ = write!(status, "{}/{}", self.bm_selected + 1, self.bm_count);
            status.draw(strip).unwrap();
        }

        if self.bm_count == 0 {
            BitmapLabel::new(
                self.bm_row_region(0),
                "No bookmarks yet",
                self.ui_fonts.body,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
            return;
        }

        let grouped_title = self.bm_group_title();

        if let Some(title) = grouped_title {
            let subtitle_text = Self::ellipsize_ascii(title, BM_MAX_TITLE_CHARS);
            BitmapLabel::new(
                self.bm_subtitle_region(),
                &subtitle_text,
                self.ui_fonts.body,
            )
            .alignment(Alignment::CenterLeft)
            .draw(strip)
            .unwrap();
        }

        let vis = self.bm_visible_lines();
        let visible = vis.min(self.bm_count.saturating_sub(self.bm_scroll));

        for i in 0..vis {
            let region = self.bm_row_region(i);
            if i < visible {
                let idx = self.bm_scroll + i;
                let entry = &self.bm_entries[idx];
                let label = if grouped_title.is_some() {
                    Self::bm_detail_label(entry)
                } else {
                    Self::bm_mixed_book_label(entry)
                };

                BitmapLabel::new(region, &label, self.ui_fonts.body)
                    .alignment(Alignment::CenterLeft)
                    .inverted(idx == self.bm_selected)
                    .draw(strip)
                    .unwrap();
            }
        }
    }
}
