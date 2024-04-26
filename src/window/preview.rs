use crossbeam_channel::{select, Receiver};
use std::mem;
use std::thread;
use std::time::Duration;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::CreateSolidBrush,
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DispatchMessageW, PeekMessageW, RegisterClassExW,
            SetLayeredWindowAttributes, TranslateMessage, HMENU, LWA_ALPHA,
            PEEK_MESSAGE_REMOVE_TYPE, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOPMOST,
            WS_EX_TRANSPARENT, WS_POPUP, WS_SYSMENU, WS_VISIBLE,
        },
    },
};

use crate::common::RGB;
use crate::window::Window;
use crate::Message;
use crate::CHANNEL;

pub fn spawn_preview_window(close_msg: Receiver<()>) {
    thread::spawn(move || unsafe {
        let hInstance = GetModuleHandleW(PCWSTR::null()).expect("failed GetModuleHandleW");

        let class_name = w!("Grout Zone Preview");

        let mut class = mem::zeroed::<WNDCLASSEXW>();
        class.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
        class.lpfnWndProc = Some(callback);
        class.hInstance = hInstance.into();
        class.lpszClassName = class_name;
        class.hbrBackground = CreateSolidBrush(RGB(0, 77, 128));

        RegisterClassExW(&class);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            class_name,
            PCWSTR::null(),
            WS_POPUP | WS_VISIBLE | WS_SYSMENU,
            0,
            0,
            0,
            0,
            HWND::default(),
            HMENU::default(),
            hInstance,
            None,
        );

        let _ = SetLayeredWindowAttributes(hwnd, COLORREF::default(), 107, LWA_ALPHA);

        let _ = &CHANNEL.0.clone().send(Message::PreviewWindow(Window(hwnd)));

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
    DefWindowProcW(hWnd, Msg, wParam, lParam)
}
