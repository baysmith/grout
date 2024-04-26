# grout
![Rust](https://github.com/tarkah/grout/workflows/Rust/badge.svg)

A simple tiling window manager for Windows, written in Rust. Inspired by Budgie's Window Shuffler grid functionality.

- [Demo](#demo)
- [Download](#download)
- [Usage](#usage)
- [Config](#config)

## Demo

Click for full video

[![Demo](https://i.imgur.com/bErviBc.gif)](https://i.imgur.com/ugPMvlA.mp4)


## Download

- Download executable from [latest release](https://github.com/tarkah/grout/releases/latest)


## Usage

- Run `grout.exe` or `cargo run`. Program will run in the background and options can be accessed by right clicking the system tray icon.
- Activate the windowing grid with hotkey `CRTL + ALT + S`.
- Increase / decrease grid rows / columns with `CTRL + arrows`.
- Hovering cursor over the grid will show a preview of that zone in the window.
- Select a window you want resized, then click on a tile in the grid. Window will resize to that zone.
- Hold `SHIFT` down while hovering after a selection, zone will increase in size across all tiles. Select again to resize to larger zone.
- Resizing can also be achieved by click-drag-release. Click & hold cursor down, drag cursor across multiple tiles and release to make selection.
- F1 - F6 can be used to toggle between saved profiles. F1 is the default profile loaded when program is first started.

## Config

```toml
# Example config file for Grout

# Margin between windows, in pixels
margins = 10

# Padding between edge of monitor and windows, in pixels
window_padding = 10

# Hotkey to activate grid. Valid modifiers are CTRL, ALT, SHIFT, WIN
hotkey = "CTRL+ALT+S"

# Hotkey to activate grid for a quick resize. Grid will automatically
# close after resize operation.
#hotkey_quick_resize = "CTRL+ALT+Q"

# Hotkey to maximize / restore the active window
#hotkey_maximize_toggle = "CTRL+ALT+X"

# Automatically launch program on startup
auto_start = false
```

- A configuration file will be created at `%APPDATA%\grout\config.toml` that can be customized. You can also open the config file from the system tray icon.

