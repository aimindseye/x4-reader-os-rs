// ui re-exports: kernel primitives + app-side font-dependent widgets
//
// kernel ui (Region, Alignment, StackFmt, statusbar constants) is
// re-exported from x4-kernel; font-dependent widgets (BitmapLabel,
// QuickMenu, ButtonFeedback) come from apps::widgets

// kernel-side primitives
pub use x4_kernel::ui::stack_fmt;
pub use x4_kernel::ui::*;

// app-side font-dependent widgets
pub use crate::apps::widgets::QuickMenu;
pub use crate::apps::widgets::bitmap_label::{BitmapDynLabel, BitmapLabel};
pub use crate::apps::widgets::button_feedback::{BUTTON_BAR_H, ButtonFeedback};
pub use crate::apps::widgets::list::ListSelection;
pub use crate::apps::widgets::quick_menu;
pub use crate::apps::widgets::selectable_row::{
    draw_selection, draw_selection_if_visible, selection_fg,
};
