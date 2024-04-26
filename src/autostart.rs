use anyhow::format_err;
use std::env;
use std::fs;
use std::mem;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::WIN32_ERROR,
        System::Registry::{
            RegCreateKeyExW, RegDeleteKeyValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
            KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
        },
    },
};

use crate::Result;

pub unsafe fn toggle_autostart_registry_key(enabled: bool) -> Result<()> {
    let mut app_path =
        dirs::config_dir().ok_or_else(|| format_err!("Failed to get config directory"))?;
    app_path.push("grout");
    app_path.push("grout.exe");

    let current_path = env::current_exe()?;
    if current_path != app_path && enabled {
        fs::copy(current_path, &app_path)?;
    }

    let app_path = app_path.to_str().unwrap_or_default();
    let key_name = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let value_name = w!("grout");

    let mut key: HKEY = mem::zeroed();

    if enabled {
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_name,
            0,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        ) == WIN32_ERROR(0)
        {
            let _ = RegSetValueExW(key, value_name, 0, REG_SZ, Some(app_path.as_bytes()));
        }
    } else {
        let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, key_name, value_name);
    }

    Ok(())
}
