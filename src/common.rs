use std::convert::TryFrom;
use std::fmt::{Display, Error, Formatter};
use std::mem;
use std::process;
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{COLORREF, HWND, POINT, RECT},
        Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromPoint, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
        },
        UI::WindowsAndMessaging::{GetCursorPos, GetForegroundWindow, MessageBoxW, MB_OK},
    },
};

use crate::str_to_wide;
use crate::window::Window;

/// x & y coordinates are relative to top left of screen
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn contains_point(self, point: (i32, i32)) -> bool {
        point.0 >= self.x
            && point.0 <= self.x + self.width
            && point.1 >= self.y
            && point.1 <= self.y + self.height
    }

    pub fn zero() -> Self {
        Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn adjust_for_border(&mut self, border: (i32, i32)) {
        self.x -= border.0;
        self.width += border.0 * 2;
        self.height += border.1;
    }
}

impl Display for Rect {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        writeln!(f, "x: {}", self.x)?;
        writeln!(f, "y: {}", self.y)?;
        writeln!(f, "width: {}", self.width)?;
        writeln!(f, "height: {}", self.height)?;

        Ok(())
    }
}

impl From<RECT> for Rect {
    fn from(rect: RECT) -> Self {
        Rect {
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        }
    }
}

impl From<Rect> for RECT {
    fn from(rect: Rect) -> Self {
        RECT {
            left: rect.x,
            top: rect.y,
            right: rect.x + rect.width,
            bottom: rect.y + rect.height,
        }
    }
}

pub fn get_foreground_window() -> Window {
    let hwnd = unsafe { GetForegroundWindow() };
    Window(hwnd)
}

pub unsafe fn get_work_area() -> Rect {
    let active_monitor = {
        let mut cursor_pos: POINT = mem::zeroed();
        let _ = GetCursorPos(&mut cursor_pos);

        MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST)
    };

    let work_area: Rect = {
        let mut info: MONITORINFOEXW = Default::default();
        info.monitorInfo.cbSize = u32::try_from(std::mem::size_of::<MONITORINFOEXW>())
            .expect("failed size_fo MONITORINFOEXW");

        let _ = GetMonitorInfoW(active_monitor, &mut info as *mut MONITORINFOEXW as *mut _);

        info.monitorInfo.rcWork.into()
    };

    work_area
}

pub unsafe fn get_active_monitor_name() -> String {
    let active_monitor = {
        let mut cursor_pos: POINT = mem::zeroed();
        let _ = GetCursorPos(&mut cursor_pos);

        MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST)
    };

    let mut info: MONITORINFOEXW = Default::default();
    info.monitorInfo.cbSize = u32::try_from(std::mem::size_of::<MONITORINFOEXW>())
        .expect("failed size_fo MONITORINFOEXW");

    let _ = GetMonitorInfoW(active_monitor, &mut info as *mut MONITORINFOEXW as *mut _);

    String::from_utf16_lossy(&info.szDevice)
}

pub fn report_and_exit(error_msg: &str) -> ! {
    show_msg_box(error_msg);
    process::exit(1)
}

pub fn show_msg_box(message: &str) {
    let mut message = str_to_wide!(message);
    let message_pwstr = PWSTR(message.as_mut_ptr());
    let hwnd: HWND = Default::default();

    unsafe {
        MessageBoxW(hwnd, message_pwstr, PCWSTR::null(), MB_OK);
    }
}

pub fn LOWORD(l: usize) -> u16 {
    (l & 0xffff) as u16
}

pub fn HIWORD(l: usize) -> u16 {
    ((l >> 16) & 0xffff) as u16
}

pub fn RGB(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(r as u32 | ((g as u32) << 8) | ((b as u32) << 16))
}
