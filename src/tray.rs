use std::mem;
use std::thread;
use windows::{
    core::{w, PCWSTR, PWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
        Graphics::Gdi::{CreateSolidBrush, HBITMAP},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::SetFocus,
            Shell::{
                ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD,
                NIM_DELETE, NOTIFYICONDATAW,
            },
            WindowsAndMessaging::{
                CheckMenuItem, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW,
                DefWindowProcW, DestroyMenu, DispatchMessageW, GetCursorPos, GetMessageW,
                InsertMenuW, MessageBoxW, PostMessageW, PostQuitMessage, RegisterClassExW,
                SendMessageW, SetForegroundWindow, SetMenuDefaultItem, SetMenuItemBitmaps,
                TrackPopupMenu, TranslateMessage, HMENU, LR_DEFAULTCOLOR, MB_ICONINFORMATION,
                MB_OK, MF_BYPOSITION, MF_CHECKED, MF_STRING, MF_UNCHECKED, SW_SHOW, TPM_LEFTALIGN,
                TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WINDOW_STYLE, WM_APP, WM_CLOSE,
                WM_COMMAND, WM_CREATE, WM_INITMENUPOPUP, WM_LBUTTONDBLCLK, WM_RBUTTONUP,
                WNDCLASSEXW, WS_EX_NOACTIVATE,
            },
        },
    },
};

use crate::autostart;
use crate::common::{show_msg_box, LOWORD, RGB};
use crate::config;
use crate::str_to_wide;
use crate::Message;
use crate::CHANNEL;
use crate::CONFIG;

const ID_ABOUT: u16 = 2000;
const ID_EXIT: u16 = 2001;
const ID_CONFIG: u16 = 2002;
const ID_AUTOSTART: u16 = 2003;
static mut MODAL_SHOWN: bool = false;

pub unsafe fn spawn_sys_tray() {
    thread::spawn(|| {
        let hInstance = GetModuleHandleW(PCWSTR::null()).expect("failed GetModuleHandleW");

        let class_name = w!("Grout Tray");

        let mut class = mem::zeroed::<WNDCLASSEXW>();
        class.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
        class.lpfnWndProc = Some(callback);
        class.hInstance = hInstance.into();
        class.lpszClassName = class_name;
        class.hbrBackground = CreateSolidBrush(RGB(0, 77, 128));

        RegisterClassExW(&class);

        CreateWindowExW(
            WS_EX_NOACTIVATE,
            class_name,
            PCWSTR::null(),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            HWND::default(),
            HMENU::default(),
            hInstance,
            None,
        );

        let mut msg = mem::zeroed();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

unsafe fn add_icon(hwnd: HWND) {
    let icon_bytes = include_bytes!("../assets/icon_32.png");

    let icon_handle =
        CreateIconFromResourceEx(icon_bytes, true, 0x0003_0000, 32, 32, LR_DEFAULTCOLOR)
            .expect("failed CreateIconFromResourceEx");

    let mut tooltip_array = [0u16; 128];
    let tooltip = "Grout";
    let mut tooltip = tooltip.encode_utf16().collect::<Vec<_>>();
    tooltip.extend(vec![0; 128 - tooltip.len()]);
    tooltip_array.swap_with_slice(&mut tooltip[..]);

    let mut icon_data: NOTIFYICONDATAW = mem::zeroed();
    icon_data.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    icon_data.hWnd = hwnd;
    icon_data.uID = 1;
    icon_data.uCallbackMessage = WM_APP;
    icon_data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    icon_data.hIcon = icon_handle;
    icon_data.szTip = tooltip_array;

    let _ = Shell_NotifyIconW(NIM_ADD, &icon_data);
}

unsafe fn remove_icon(hwnd: HWND) {
    let mut icon_data: NOTIFYICONDATAW = mem::zeroed();
    icon_data.hWnd = hwnd;
    icon_data.uID = 1;

    let _ = Shell_NotifyIconW(NIM_DELETE, &icon_data);
}

unsafe fn show_popup_menu(hwnd: HWND) {
    if MODAL_SHOWN {
        return;
    }

    let menu = CreatePopupMenu().expect("failed CreatePopupMenu");

    let about = w!("About...");
    let auto_start = w!("Launch at startup");
    let open_config = w!("Open Config");
    let exit = w!("Exit");

    let _ = InsertMenuW(menu, 0, MF_BYPOSITION | MF_STRING, ID_ABOUT as usize, about);

    let _ = InsertMenuW(
        menu,
        1,
        MF_BYPOSITION | MF_STRING,
        ID_AUTOSTART as usize,
        auto_start,
    );

    let _ = SetMenuItemBitmaps(
        menu,
        1,
        MF_BYPOSITION,
        HBITMAP::default(),
        HBITMAP::default(),
    );

    let checked = if CONFIG.lock().unwrap().auto_start {
        MF_CHECKED
    } else {
        MF_UNCHECKED
    };

    CheckMenuItem(menu, 1, (MF_BYPOSITION | checked).0);

    let _ = InsertMenuW(
        menu,
        2,
        MF_BYPOSITION | MF_STRING,
        ID_CONFIG as usize,
        open_config,
    );

    let _ = InsertMenuW(menu, 3, MF_BYPOSITION | MF_STRING, ID_EXIT as usize, exit);

    let _ = SetMenuDefaultItem(menu, ID_ABOUT as u32, 0);
    SetFocus(hwnd);
    SendMessageW(
        hwnd,
        WM_INITMENUPOPUP,
        WPARAM(menu.0 as usize),
        LPARAM::default(),
    );

    let mut point: POINT = mem::zeroed();
    let _ = GetCursorPos(&mut point);

    let cmd = TrackPopupMenu(
        menu,
        TPM_LEFTALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
        point.x,
        point.y,
        0,
        hwnd,
        None,
    );

    SendMessageW(hwnd, WM_COMMAND, WPARAM(cmd.0 as usize), LPARAM::default());

    let _ = DestroyMenu(menu);
}

unsafe fn show_about() {
    let title = w!("About");

    let msg = format!(
        "Grout - v{}\n\nCopyright © 2024 Bradley Smith",
        env!("CARGO_PKG_VERSION")
    );

    let mut msg = str_to_wide!(msg);
    let msg_pwstr = PWSTR(msg.as_mut_ptr());

    MessageBoxW(
        HWND::default(),
        msg_pwstr,
        title,
        MB_ICONINFORMATION | MB_OK,
    );
}

unsafe extern "system" fn callback(
    hWnd: HWND,
    Msg: u32,
    wParam: WPARAM,
    lParam: LPARAM,
) -> LRESULT {
    match Msg {
        WM_CREATE => {
            add_icon(hWnd);
            return LRESULT::default();
        }
        WM_CLOSE => {
            remove_icon(hWnd);
            PostQuitMessage(0);
            let _ = &CHANNEL.0.clone().send(Message::Exit);
        }
        WM_COMMAND => {
            if MODAL_SHOWN {
                return LRESULT(1);
            }

            match LOWORD(wParam.0) {
                ID_ABOUT => {
                    MODAL_SHOWN = true;

                    show_about();

                    MODAL_SHOWN = false;
                }
                ID_AUTOSTART => {
                    if let Err(e) = config::toggle_autostart() {
                        show_msg_box(&format!(
                            "Error while toggling autostart from system tray.\n\nErr: {}",
                            e
                        ))
                    };

                    let mut config = CONFIG.lock().unwrap();
                    match config::load_config() {
                        Ok(_config) => *config = _config,
                        Err(e) => show_msg_box(&format!("Error loading config while toggling autostart from system tray. Check config file for formatting errors.\n\nErr: {}", e)),
                    }

                    if let Err(e) = autostart::toggle_autostart_registry_key(config.auto_start) {
                        show_msg_box(&format!(
                            "Error updating registry while toggling autostart from system tray.\n\nErr: {}",
                            e
                        ))
                    };
                }
                ID_CONFIG => {
                    if let Some(mut config_path) = dirs::config_dir() {
                        config_path.push("grout");
                        config_path.push("config.toml");

                        if config_path.exists() {
                            let operation = w!("open");
                            let mut config_path = str_to_wide!(config_path.to_str().unwrap());
                            let config_path_pwstr = PWSTR(config_path.as_mut_ptr());

                            ShellExecuteW(
                                hWnd,
                                operation,
                                config_path_pwstr,
                                PCWSTR::null(),
                                PCWSTR::null(),
                                SW_SHOW,
                            );
                        }
                    }
                }
                ID_EXIT => {
                    let _ = PostMessageW(hWnd, WM_CLOSE, WPARAM::default(), LPARAM::default());
                }
                _ => {}
            }

            return LRESULT(0);
        }
        WM_APP => {
            match lParam.0 as u32 {
                WM_LBUTTONDBLCLK => show_about(),
                WM_RBUTTONUP => {
                    let _ = SetForegroundWindow(hWnd);
                    show_popup_menu(hWnd);
                    let _ = PostMessageW(hWnd, WM_APP + 1, WPARAM::default(), LPARAM::default());
                }
                _ => {}
            }

            return LRESULT(0);
        }
        _ => {}
    }

    DefWindowProcW(hWnd, Msg, wParam, lParam)
}
