use std::mem;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        GetWindowInfo, GetWindowRect, SetWindowPos, ShowWindow, SWP_NOACTIVATE, SW_RESTORE,
        WINDOWINFO, WINDOW_EX_STYLE, WINDOW_STYLE,
    },
};

use crate::common::Rect;

mod grid;
pub use grid::spawn_grid_window;

mod preview;
pub use preview::spawn_preview_window;

#[derive(Clone, Copy, Default, Debug)]
pub struct Window(pub HWND);

unsafe impl Send for Window {}

impl Window {
    pub fn rect(self) -> Rect {
        unsafe {
            let mut rect = mem::zeroed();

            let _ = GetWindowRect(self.0, &mut rect);

            rect.into()
        }
    }

    pub fn set_pos(&mut self, rect: Rect, insert_after: Option<Window>) {
        unsafe {
            let _ = SetWindowPos(
                self.0,
                insert_after.unwrap_or_default().0,
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                SWP_NOACTIVATE,
            );
        }
    }

    pub unsafe fn info(self) -> WindowInfo {
        let mut info: WINDOWINFO = mem::zeroed();
        info.cbSize = mem::size_of::<WINDOWINFO>() as u32;

        let _ = GetWindowInfo(self.0, &mut info);

        info.into()
    }

    pub fn transparent_border(self) -> (i32, i32) {
        let info = unsafe { self.info() };

        let x = {
            (info.window_rect.x - info.client_rect.x)
                + (info.window_rect.width - info.client_rect.width)
        };

        let y = {
            (info.window_rect.y - info.client_rect.y)
                + (info.window_rect.height - info.client_rect.height)
        };

        (x, y)
    }

    pub fn restore(&mut self) {
        unsafe {
            let _ = ShowWindow(self.0, SW_RESTORE);
        };
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Window) -> bool {
        self.0 == other.0
    }
}

#[derive(Debug)]
pub struct WindowInfo {
    pub window_rect: Rect,
    pub client_rect: Rect,
    pub styles: WINDOW_STYLE,
    pub extended_styles: WINDOW_EX_STYLE,
    pub x_borders: u32,
    pub y_borders: u32,
}

impl From<WINDOWINFO> for WindowInfo {
    fn from(info: WINDOWINFO) -> Self {
        WindowInfo {
            window_rect: info.rcWindow.into(),
            client_rect: info.rcClient.into(),
            styles: info.dwStyle,
            extended_styles: info.dwExStyle,
            x_borders: info.cxWindowBorders,
            y_borders: info.cxWindowBorders,
        }
    }
}
