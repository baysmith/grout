use std::mem;
use std::thread;
use windows::Win32::{
    Foundation::HWND,
    UI::{
        Input::KeyboardAndMouse::{
            GetKeyboardLayout, RegisterHotKey, VkKeyScanExW, HOT_KEY_MODIFIERS, MOD_ALT,
            MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
        },
        WindowsAndMessaging::{DispatchMessageW, GetMessageW, TranslateMessage, WM_HOTKEY},
    },
};

use crate::common::report_and_exit;
use crate::Message;
use crate::CHANNEL;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum HotkeyType {
    Main,
    QuickResize,
    Maximize,
    NavigateRight,
    NavigateLeft,
    NavigateDown,
    NavigateUp,
    Exit,
}

pub fn spawn_hotkey_thread(hotkey_str: &str, hotkey_type: HotkeyType) {
    let mut hotkey: Vec<String> = hotkey_str
        .split('+')
        .map(|s| s.trim().to_string())
        .collect();

    if hotkey.len() < 2 || hotkey.len() > 5 {
        report_and_exit(&format!(
            "Invalid hotkey <{}>: Combination must be between 2 to 5 keys long.",
            hotkey_str
        ));
    }

    let virtual_key_char = hotkey.pop().unwrap().chars().next().unwrap();

    let hotkey_str = hotkey_str.to_owned();
    thread::spawn(move || unsafe {
        let sender = &CHANNEL.0.clone();
        let hwnd: HWND = Default::default();

        let result = RegisterHotKey(
            hwnd,
            0,
            compile_modifiers(&hotkey, &hotkey_str) | MOD_NOREPEAT,
            get_vkcode(virtual_key_char),
        );

        if result.is_err() {
            report_and_exit(&format!("Failed to assign hot key <{}>. Either program is already running or hotkey is already assigned in another program.", hotkey_str));
        }

        let mut msg = mem::zeroed();
        let hwnd: HWND = Default::default();
        while GetMessageW(&mut msg, hwnd, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);

            if msg.message == WM_HOTKEY {
                let _ = sender.send(Message::HotkeyPressed(hotkey_type));
            }
        }
    });
}

fn compile_modifiers(activators: &[String], hotkey_str: &str) -> HOT_KEY_MODIFIERS {
    let mut code: HOT_KEY_MODIFIERS = Default::default();
    for key in activators {
        match key.as_str() {
            "ALT" => code |= MOD_ALT,
            "CTRL" => code |= MOD_CONTROL,
            "SHIFT" => code |= MOD_SHIFT,
            "WIN" => code |= MOD_WIN,
            _ => report_and_exit(&format!("Invalid hotkey <{}>: Unidentified modifier in hotkey combination. Valid modifiers are CTRL, ALT, SHIFT, WIN.", hotkey_str))
        }
    }
    code
}

unsafe fn get_vkcode(key_char: char) -> u32 {
    let keyboard_layout = GetKeyboardLayout(0);
    let vk_code = VkKeyScanExW(key_char as u16, keyboard_layout);

    if vk_code == -1 {
        report_and_exit(&format!("Invalid key {} in hotkey combination.", key_char));
    }

    vk_code.to_be_bytes()[1] as u32
}
