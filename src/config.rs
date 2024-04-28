use std::fs::{create_dir_all, write, File};
use std::io::Read;

use anyhow::format_err;
use csscolorparser::Color;
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

# Automatically launch program on startup
auto_start = false

# Optional hotkeys
#[optional_hotkeys]
# Hotkey to activate grid for a quick resize. Grid will automatically close after resize operation.
#quick_resize = "CTRL+ALT+Q"

# Hotkey to maximize / restore the active window
#maximize_toggle = "CTRL+ALT+X"

# Navigate foreground window with hotkeys
#[optional_hotkeys.navigate]
#left = "ALT+H"
#down = "ALT+J"
#up = "ALT+K"
#right = "ALT+L"

# Optional customization of grid dimensions
#[grid]
#tile_width = 48
#tile_height = 48
#margins = 3

# Optional customization of colors
#[colors]
#tile = "rgb(178, 178, 178)"
#tile_hovered = "rgb(0, 100, 148)"
#tile_selected = "rgb(0, 77, 128)"
#tile_frame = "rgb(0, 0, 0)"
#grid_background = "rgba(44, 44, 44, 1.0)"
#preview = "rgba(0, 77, 128, 0.42)"
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

    let mut config_doc = config_str
        .parse::<DocumentMut>()
        .expect("invalid config.toml");
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
pub struct CustomGridConfig {
    pub tile_width: Option<u32>,
    pub tile_height: Option<u32>,
    pub margins: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomColors {
    pub tile: Option<Color>,
    pub tile_hovered: Option<Color>,
    pub tile_selected: Option<Color>,
    pub tile_frame: Option<Color>,
    pub grid_background: Option<Color>,
    pub preview: Option<Color>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NavigateHotkeys {
    pub left: Option<String>,
    pub down: Option<String>,
    pub up: Option<String>,
    pub right: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptionalHotkeys {
    pub quick_resize: Option<String>,
    pub maximize_toggle: Option<String>,
    pub navigate: Option<NavigateHotkeys>,
    pub quick_exit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub margins: u8,
    pub window_padding: u8,
    pub hotkey: String,
    pub optional_hotkeys: Option<OptionalHotkeys>,
    pub auto_start: bool,
    pub grid: Option<CustomGridConfig>,
    pub colors: Option<CustomColors>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            margins: 10,
            window_padding: 10,
            hotkey: "CTRL+ALT+S".to_string(),
            optional_hotkeys: None,
            auto_start: false,
            grid: None,
            colors: None,
        }
    }
}
