use crossbeam_channel::{select, Receiver};
use csscolorparser::Color;
use std::mem;
use std::thread;
use std::time::Duration;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{CreateSolidBrush, InvalidateRect},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::WM_MOUSELEAVE,
            Input::KeyboardAndMouse::{
                VIRTUAL_KEY, VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5,
                VK_F6, VK_LEFT, VK_RIGHT, VK_SHIFT, VK_UP,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DispatchMessageW, LoadCursorW, PeekMessageW,
                RegisterClassExW, SendMessageW, SetLayeredWindowAttributes, TranslateMessage,
                HMENU, IDC_ARROW, LWA_ALPHA, PEEK_MESSAGE_REMOVE_TYPE, WM_KEYDOWN, WM_KEYUP,
                WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT, WNDCLASSEXW, WS_EX_LAYERED,
                WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
            },
        },
    },
};

use crate::common::{color_to_colorref, get_work_area, Rect, HIWORD, LOWORD};
use crate::window::Window;
use crate::Message;
use crate::{CHANNEL, GRID};

pub fn spawn_grid_window(close_msg: Receiver<()>, background: Color) {
    thread::spawn(move || unsafe {
        let hInstance = GetModuleHandleW(PCWSTR::null()).expect("failed GetModuleHandleW");

        let class_name = w!("Grout Zone Grid");

        let mut class = mem::zeroed::<WNDCLASSEXW>();
        class.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
        class.lpfnWndProc = Some(callback);
        class.hInstance = hInstance.into();
        class.lpszClassName = class_name;
        class.hCursor = LoadCursorW(HINSTANCE::default(), IDC_ARROW).expect("failed LoadCursorW");

        let alpha = background.to_rgba8()[3];
        class.hbrBackground = CreateSolidBrush(color_to_colorref(&background));

        RegisterClassExW(&class);

        let work_area = get_work_area();
        let dimensions = GRID.lock().unwrap().dimensions();

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            PCWSTR::null(),
            WS_POPUP,
            work_area.width / 2 - dimensions.0 as i32 / 2 + work_area.x,
            work_area.height / 2 - dimensions.1 as i32 / 2 + work_area.y,
            dimensions.0 as i32,
            dimensions.1 as i32,
            HWND::default(),
            HMENU::default(),
            hInstance,
            None,
        );

        let _ = SetLayeredWindowAttributes(hwnd, COLORREF::default(), alpha, LWA_ALPHA);

        let _ = &CHANNEL.0.clone().send(Message::GridWindow(Window(hwnd)));

        let mut msg = mem::zeroed();
        loop {
            if PeekMessageW(&mut msg, HWND::default(), 0, 0, PEEK_MESSAGE_REMOVE_TYPE(1)).into() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            };

            select! {
                recv(close_msg) -> _ => {
                    break;
                }
                default(Duration::from_millis(10)) => {}
            }
        }
    });
}

unsafe extern "system" fn callback(
    hWnd: HWND,
    Msg: u32,
    wParam: WPARAM,
    lParam: LPARAM,
) -> LRESULT {
    let sender = &CHANNEL.0.clone();

    let repaint = match Msg {
        WM_PAINT => {
            GRID.lock().unwrap().draw(Window(hWnd));
            false
        }
        WM_KEYDOWN => match VIRTUAL_KEY(LOWORD(wParam.0)) {
            VK_ESCAPE => {
                let _ = sender.send(Message::CloseWindows);
                false
            }
            VK_CONTROL => {
                GRID.lock().unwrap().control_down = true;
                false
            }
            VK_SHIFT => {
                GRID.lock().unwrap().shift_down = true;
                false
            }
            VK_RIGHT => {
                if GRID.lock().unwrap().control_down {
                    GRID.lock().unwrap().add_column();
                    GRID.lock().unwrap().reposition();
                }
                false
            }
            VK_LEFT => {
                if GRID.lock().unwrap().control_down {
                    GRID.lock().unwrap().remove_column();
                    GRID.lock().unwrap().reposition();
                }
                false
            }
            VK_UP => {
                if GRID.lock().unwrap().control_down {
                    GRID.lock().unwrap().add_row();
                    GRID.lock().unwrap().reposition();
                }
                false
            }
            VK_DOWN => {
                if GRID.lock().unwrap().control_down {
                    GRID.lock().unwrap().remove_row();
                    GRID.lock().unwrap().reposition();
                }
                false
            }
            _ => false,
        },
        WM_KEYUP => match VIRTUAL_KEY(LOWORD(wParam.0)) {
            VK_CONTROL => {
                GRID.lock().unwrap().control_down = false;
                false
            }
            VK_SHIFT => {
                GRID.lock().unwrap().shift_down = false;
                false
            }
            VK_F1 => {
                let _ = sender.send(Message::ProfileChange("Default"));
                false
            }
            VK_F2 => {
                let _ = sender.send(Message::ProfileChange("Profile2"));
                false
            }
            VK_F3 => {
                let _ = sender.send(Message::ProfileChange("Profile3"));
                false
            }
            VK_F4 => {
                let _ = sender.send(Message::ProfileChange("Profile4"));
                false
            }
            VK_F5 => {
                let _ = sender.send(Message::ProfileChange("Profile5"));
                false
            }
            VK_F6 => {
                let _ = sender.send(Message::ProfileChange("Profile6"));
                false
            }
            _ => false,
        },
        WM_MOUSEMOVE => {
            let x = LOWORD(lParam.0 as usize) as i32;
            let y = HIWORD(lParam.0 as usize) as i32;

            let _ = sender.send(Message::TrackMouse(Window(hWnd)));

            if let Some(rect) = GRID.lock().unwrap().highlight_tiles((x, y)) {
                let _ = sender.send(Message::HighlightZone(rect));

                true
            } else {
                false
            }
        }
        WM_LBUTTONDOWN => {
            let x = LOWORD(lParam.0 as usize) as i32;
            let y = HIWORD(lParam.0 as usize) as i32;

            let mut grid = GRID.lock().unwrap();

            let repaint = grid.select_tile((x, y));

            grid.cursor_down = true;

            repaint
        }
        WM_LBUTTONUP => {
            let mut grid = GRID.lock().unwrap();

            let repaint = if let Some(mut rect) = grid.selected_area() {
                if let Some(mut active_window) = grid.active_window {
                    if grid.previous_resize != Some((active_window, rect)) {
                        active_window.restore();

                        rect.adjust_for_border(active_window.transparent_border());

                        active_window.set_pos(rect, None);

                        grid.previous_resize = Some((active_window, rect));

                        if grid.quick_resize {
                            let _ = sender.send(Message::CloseWindows);
                        }
                    }

                    grid.unselect_all_tiles();
                }

                true
            } else {
                false
            };

            grid.cursor_down = false;

            repaint
        }
        WM_MOUSELEAVE => {
            GRID.lock().unwrap().unhighlight_all_tiles();

            let _ = sender.send(Message::MouseLeft);
            let _ = sender.send(Message::HighlightZone(Rect::zero()));

            true
        }
        _ => false,
    };

    if repaint {
        let dimensions = GRID.lock().unwrap().dimensions();
        let rect = Rect {
            x: 0,
            y: 0,
            width: dimensions.0 as i32,
            height: dimensions.1 as i32,
        };

        let r: RECT = rect.into();
        let _ = InvalidateRect(hWnd, Some(&r as *const RECT), false);
        SendMessageW(hWnd, WM_PAINT, WPARAM::default(), LPARAM::default());
    }

    DefWindowProcW(hWnd, Msg, wParam, lParam)
}
