use std::fs::{create_dir_all, write, File};
use std::io::Read;

use anyhow::format_err;
use serde::{Deserialize, Serialize};
use toml_edit::{value, DocumentMut};

use crate::Result;

static EXAMPLE_CONFIG: &str = r#"
# Example config file for Grout

# Margin between windows, in pixels
margins = 10

# Padding between edge of monitor and windows, in pixels
window_padding = 10

# Hotkey to activate grid. Valid modifiers are CTRL, ALT, SHIFT, WIN
hotkey = "CTRL+ALT+S"

# Hotkey to activate grid for a quick resize. Grid will automatically close after resize operation.
#hotkey_quick_resize = "CTRL+ALT+Q"

# Hotkey to maximize / restore the active window
#hotkey_maximize_toggle = "CTRL+ALT+X"

# Automatically launch program on startup
auto_start = false
"#;

pub fn load_config() -> Result<Config> {
    let mut config_path =
        dirs::config_dir().ok_or_else(|| format_err!("Failed to get config directory"))?;
    config_path.push("grout");

    if !config_path.exists() {
        create_dir_all(&config_path)?;
    }

    config_path.push("config.toml");
    if !config_path.exists() {
        write(&config_path, EXAMPLE_CONFIG)?;
    }

    config::Config::builder()
        .add_source(config::File::new(
            config_path.to_str().expect("invalid config path"),
            config::FileFormat::Toml,
        ))
        .build()?
        .try_deserialize::<Config>()
        .map_err(|e| e.into())
}

pub fn toggle_autostart() -> Result<()> {
    let mut config_path =
        dirs::config_dir().ok_or_else(|| format_err!("Failed to get config directory"))?;
    config_path.push("grout");
    config_path.push("config.toml");

    let mut config = File::open(&config_path)?;
    let mut config_str = String::new();

    config.read_to_string(&mut config_str)?;

    let mut config_doc = config_str.parse::<DocumentMut>().expect("invalid config.toml");
    let enabled = if let Some(auto_start) = config_doc["auto_start"].as_bool() {
        !auto_start
    } else {
        false
    };
    config_doc["auto_start"] = value(enabled);

    write(&config_path, config_doc.to_string())?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub margins: u8,
    pub window_padding: u8,
    pub hotkey: String,
    pub hotkey_quick_resize: Option<String>,
    pub hotkey_maximize_toggle: Option<String>,
    pub auto_start: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            margins: 10,
            window_padding: 10,
            hotkey: "CTRL+ALT+S".to_string(),
            hotkey_quick_resize: None,
            hotkey_maximize_toggle: None,
            auto_start: false,
        }
    }
}
