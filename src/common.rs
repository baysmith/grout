use anyhow::Result;
use csscolorparser::Color;
use std::fmt::{Display, Error, Formatter};
use std::mem;
use std::process;
use std::{convert::TryFrom, ffi::c_void};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{BOOL, COLORREF, HWND, LPARAM, POINT, RECT},
        Graphics::{
            Dwm::{
                DwmGetWindowAttribute, DWMWA_CLOAKED, DWM_CLOAKED_APP, DWM_CLOAKED_INHERITED,
                DWM_CLOAKED_SHELL,
            },
            Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST},
        },
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION,
        },
        UI::{
            Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_MOUSE},
            WindowsAndMessaging::{
                EnumWindows, GetCursorPos, GetForegroundWindow, GetWindowLongW, GetWindowRect,
                GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible,
                MessageBoxW, SetForegroundWindow, SetWindowPos, GWL_EXSTYLE, HWND_TOP, MB_OK,
                SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, WINDOW_EX_STYLE, WS_EX_TOOLWINDOW,
            },
        },
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

pub fn color_to_colorref(color: &Color) -> COLORREF {
    let [r, g, b, ..] = color.to_rgba8();
    COLORREF(r as u32 | ((g as u32) << 8) | ((b as u32) << 16))
}

unsafe fn window_process_and_thread_id(hwnd: HWND) -> (u32, u32) {
    let mut process_id: u32 = 0;
    let thread_id = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    (process_id, thread_id)
}

#[allow(dead_code)]
unsafe fn window_exe(hwnd: HWND) -> String {
    let mut len = 260_u32;
    let mut path: Vec<u16> = vec![0; len as usize];
    let path_pwstr = PWSTR(path.as_mut_ptr());
    let (process_id, _) = window_process_and_thread_id(hwnd);
    if let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION, false, process_id) {
        if QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, path_pwstr, &mut len).is_ok() {
            String::from_utf16(&path[..len as usize]).unwrap()
        } else {
            String::new()
        }
    } else {
        String::new()
    }
}

#[allow(dead_code)]
pub unsafe fn window_title(hwnd: HWND) -> String {
    let mut title = [0; 512];
    GetWindowTextW(hwnd, &mut title[..]);
    let title_len = title.iter().position(|b| *b == 0).unwrap();
    String::from_utf16_lossy(&title[..title_len])
}

unsafe fn window_is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED,
        &mut cloaked as *mut u32 as *mut c_void,
        u32::try_from(std::mem::size_of::<u32>()).unwrap(),
    )
    .unwrap();

    cloaked & DWM_CLOAKED_APP != 0
        || cloaked & DWM_CLOAKED_SHELL != 0
        || cloaked & DWM_CLOAKED_INHERITED != 0
}

unsafe fn window_rect(hwnd: HWND) -> RECT {
    let mut rect = std::mem::zeroed();
    let _ = GetWindowRect(hwnd, &mut rect);
    rect
}

pub fn nav_window_list() -> Result<Vec<HWND>> {
    let mut window_list: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_windows_callback),
            LPARAM(&mut window_list as *mut _ as isize),
        );
    }
    Ok(window_list)
}

pub enum OrderingDirection {
    Horizontal,
    Vertical,
}

pub fn order_window_list(window_list: &mut [HWND], direction: OrderingDirection) {
    unsafe {
        match direction {
            OrderingDirection::Horizontal => {
                window_list.sort_by(|a, b| {
                    let a_rect = window_rect(*a);
                    let b_rect = window_rect(*b);
                    if a_rect.left == b_rect.left {
                        if a_rect.top == b_rect.top {
                            if a_rect.right == b_rect.right {
                                if a_rect.bottom == b_rect.bottom {
                                    let (a_process_id, _) = window_process_and_thread_id(*a);
                                    let (b_process_id, _) = window_process_and_thread_id(*b);
                                    a_process_id.cmp(&b_process_id)
                                } else {
                                    a_rect.bottom.cmp(&b_rect.bottom)
                                }
                            } else {
                                a_rect.right.cmp(&b_rect.right)
                            }
                        } else {
                            a_rect.top.cmp(&b_rect.top)
                        }
                    } else {
                        a_rect.left.cmp(&b_rect.left)
                    }
                });
            }
            OrderingDirection::Vertical => {
                window_list.sort_by(|a, b| {
                    let a_rect = window_rect(*a);
                    let b_rect = window_rect(*b);
                    if a_rect.top == b_rect.top {
                        if a_rect.left == b_rect.left {
                            if a_rect.bottom == b_rect.bottom {
                                if a_rect.right == b_rect.right {
                                    let (a_process_id, _) = window_process_and_thread_id(*a);
                                    let (b_process_id, _) = window_process_and_thread_id(*b);
                                    a_process_id.cmp(&b_process_id)
                                } else {
                                    a_rect.right.cmp(&b_rect.right)
                                }
                            } else {
                                a_rect.bottom.cmp(&b_rect.bottom)
                            }
                        } else {
                            a_rect.left.cmp(&b_rect.left)
                        }
                    } else {
                        a_rect.top.cmp(&b_rect.top)
                    }
                });
            }
        }
    }
}

pub fn next_window(windows: &[HWND]) -> Option<&HWND> {
    let current = get_foreground_window();
    if let Some(index) = windows.iter().position(|w| *w == current.0) {
        if index == windows.len() - 1 {
            windows.first()
        } else {
            windows.get(index + 1)
        }
    } else {
        windows.last()
    }
}

pub fn previous_window(windows: &[HWND]) -> Option<&HWND> {
    let current = get_foreground_window();
    if let Some(index) = windows.iter().position(|w| *w == current.0) {
        if index == 0 {
            windows.last()
        } else {
            windows.get(index - 1)
        }
    } else {
        windows.first()
    }
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let window_list = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };

    let is_visible: bool = IsWindowVisible(hwnd).into();
    let is_iconic: bool = IsIconic(hwnd).into();
    let ex_style = WINDOW_EX_STYLE(GetWindowLongW(hwnd, GWL_EXSTYLE) as u32);
    let is_tool_window = ex_style.contains(WS_EX_TOOLWINDOW);
    let is_window: bool = IsWindow(hwnd).into();
    let is_cloaked = window_is_cloaked(hwnd);

    if is_window && is_visible && !is_tool_window && !is_cloaked && !is_iconic {
        window_list.push(hwnd);
    }

    true.into()
}

pub fn focus_window(hwnd: HWND) -> bool {
    let event = [INPUT {
        r#type: INPUT_MOUSE,
        ..Default::default()
    }];

    unsafe {
        SendInput(&event, std::mem::size_of::<INPUT>() as i32);
        let _ = SetWindowPos(
            hwnd,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
        SetForegroundWindow(hwnd).into()
    }
}
