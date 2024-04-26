use crossbeam_channel::{select, Receiver};
use std::mem;
use std::thread;
use std::time::Duration;
use windows::Win32::{
    Foundation::{HMODULE, HWND},
    UI::{
        Accessibility::{SetWinEventHook, HWINEVENTHOOK},
        WindowsAndMessaging::{
            DispatchMessageW, PeekMessageW, TranslateMessage, EVENT_SYSTEM_FOREGROUND,
            PEEK_MESSAGE_REMOVE_TYPE, WINEVENT_OUTOFCONTEXT,
        },
    },
};

use crate::common::get_active_monitor_name;
use crate::window::Window;
use crate::Message;
use crate::CHANNEL;

pub fn spawn_foreground_hook(close_msg: Receiver<()>) {
    thread::spawn(move || unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            HMODULE::default(),
            Some(callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );

        let mut msg = mem::zeroed();
        let hwnd: HWND = Default::default();
        loop {
            if PeekMessageW(&mut msg, hwnd, 0, 1, PEEK_MESSAGE_REMOVE_TYPE(0)).into() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            };

            select! {
                recv(close_msg) -> _ => break,
                default(Duration::from_millis(10)) => {}
            }
        }
    });
}

pub fn spawn_track_monitor_thread(close_msg: Receiver<()>) {
    thread::spawn(move || unsafe {
        let sender = &CHANNEL.0.clone();

        let mut previous_monitor = get_active_monitor_name();

        loop {
            let current_monitor = get_active_monitor_name();

            if current_monitor != previous_monitor {
                previous_monitor = current_monitor.clone();

                let _ = sender.send(Message::MonitorChange);
            }

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
    _hWinEventHook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _idObject: i32,
    _idChild: i32,
    _idEventThread: u32,
    _dwmsEventTime: u32,
) {
    let sender = &CHANNEL.0.clone();
    let _ = sender.send(Message::ActiveWindowChange(Window(hwnd)));
}
