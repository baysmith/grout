use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::mem;
use windows::Win32::{
    Foundation::COLORREF,
    Graphics::Gdi::{
        BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, FrameRect, HBRUSH, HDC,
        PAINTSTRUCT,
    },
};

use crate::common::{color_to_colorref, get_active_monitor_name, get_work_area, Rect};
use crate::config::Config;
use crate::window::Window;
use crate::ACTIVE_PROFILE;

pub struct Grid {
    pub shift_down: bool,
    pub control_down: bool,
    pub cursor_down: bool,
    pub selected_tile: Option<(usize, usize)>,
    pub hovered_tile: Option<(usize, usize)>,
    pub active_window: Option<Window>,
    pub grid_window: Option<Window>,
    pub previous_resize: Option<(Window, Rect)>,
    pub quick_resize: bool,
    grid_margins: u8,
    zone_margins: u8,
    border_margins: u8,
    tiles: Vec<Vec<Tile>>, // tiles[row][column]
    active_config: GridConfigKey,
    configs: GridConfigs,
    tile_width: u32,
    tile_height: u32,
    tile_frame_color: COLORREF,
    tile_normal_color: COLORREF,
    tile_hovered_color: COLORREF,
    tile_selected_color: COLORREF,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct GridConfig {
    rows: usize,
    columns: usize,
}

impl Default for GridConfig {
    fn default() -> Self {
        GridConfig {
            rows: 2,
            columns: 2,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Debug)]
pub struct GridConfigKey {
    monitor: String,
    profile: String,
}

impl Default for GridConfigKey {
    fn default() -> Self {
        let monitor = unsafe { get_active_monitor_name() };
        let profile = ACTIVE_PROFILE.lock().unwrap().clone();

        GridConfigKey { monitor, profile }
    }
}

pub type GridConfigs = HashMap<GridConfigKey, GridConfig>;
pub trait GridCache {
    fn load() -> GridConfigs;
    fn save(&self);
}

impl GridCache for GridConfigs {
    fn load() -> GridConfigs {
        if let Some(mut config_path) = dirs::config_dir() {
            config_path.push("grout");
            config_path.push("cache");

            if !config_path.exists() {
                let _ = fs::create_dir_all(&config_path);
            }

            config_path.push("grid.ron");

            if let Ok(file) = fs::File::open(config_path) {
                if let Ok(config) = ron::de::from_reader(file) {
                    return config;
                }
            }
        }

        let mut config = HashMap::new();
        config.insert(GridConfigKey::default(), GridConfig::default());
        config
    }

    fn save(&self) {
        if let Some(mut config_path) = dirs::config_dir() {
            config_path.push("grout");
            config_path.push("cache");
            config_path.push("grid.ron");

            if let Ok(serialized) = ron::ser::to_string(&self) {
                let _ = fs::write(config_path, serialized);
            }
        }
    }
}

impl From<&Config> for Grid {
    fn from(config: &Config) -> Self {
        let mut tile_width = 48;
        let mut tile_height = 48;
        let mut grid_margins = 3;
        if let Some(grid_config) = &config.grid {
            if let Some(width) = grid_config.tile_width {
                tile_width = width;
            }
            if let Some(height) = grid_config.tile_height {
                tile_height = height;
            }
            if let Some(margins) = grid_config.margins {
                grid_margins = margins;
            }
        }
        let mut grid = Grid {
            zone_margins: config.margins,
            border_margins: config.window_padding,
            tile_width,
            tile_height,
            grid_margins,
            ..Default::default()
        };

        if let Some(colors) = &config.colors {
            if let Some(color) = &colors.tile {
                grid.tile_normal_color = color_to_colorref(&color.clone());
            }
            if let Some(color) = &colors.tile_hovered {
                grid.tile_hovered_color = color_to_colorref(&color.clone());
            }
            if let Some(color) = &colors.tile_selected {
                grid.tile_selected_color = color_to_colorref(&color.clone());
            }
            if let Some(color) = &colors.tile_frame {
                grid.tile_frame_color = color_to_colorref(&color.clone());
            }
        }

        let Grid {
            tile_normal_color,
            tile_hovered_color,
            tile_selected_color,
            tile_frame_color,
            ..
        } = grid;
        grid.tiles.iter_mut().for_each(|row| {
            row.iter_mut().for_each(|tile| {
                tile.normal_color = tile_normal_color;
                tile.hovered_color = tile_hovered_color;
                tile.selected_color = tile_selected_color;
                tile.frame_color = tile_frame_color;
            })
        });
        grid
    }
}

impl Default for Grid {
    fn default() -> Self {
        let configs = GridConfigs::load();
        let active_config = GridConfigKey::default();

        let default_config = configs.get(&active_config).cloned().unwrap_or_default();

        let rows = default_config.rows;
        let columns = default_config.columns;

        Grid {
            shift_down: false,
            control_down: false,
            cursor_down: false,
            selected_tile: None,
            hovered_tile: None,
            active_window: None,
            grid_window: None,
            previous_resize: None,
            quick_resize: false,
            grid_margins: 3,
            zone_margins: 10,
            border_margins: 10,
            tiles: vec![vec![Tile::default(); columns]; rows],
            active_config,
            configs,
            tile_width: 48,
            tile_height: 48,
            tile_normal_color: color_to_colorref(&[178, 178, 178, 255].into()),
            tile_hovered_color: color_to_colorref(&[0, 100, 148, 255].into()),
            tile_selected_color: color_to_colorref(&[0, 77, 128, 255].into()),
            tile_frame_color: color_to_colorref(&[0, 0, 0, 255].into()),
        }
    }
}

impl Grid {
    pub fn reset(&mut self) {
        self.shift_down = false;
        self.control_down = false;
        self.cursor_down = false;
        self.selected_tile = None;
        self.hovered_tile = None;
        self.grid_window = None;
        self.quick_resize = false;

        self.tiles.iter_mut().for_each(|row| {
            row.iter_mut().for_each(|tile| {
                tile.selected = false;
                tile.hovered = false;
            })
        });
    }

    fn save_config(&mut self) {
        let rows = self.rows();
        let columns = self.columns();

        if let Some(grid_config) = self.configs.get_mut(&self.active_config) {
            grid_config.rows = rows;
            grid_config.columns = columns;
        } else {
            self.configs
                .insert(self.active_config.clone(), GridConfig { rows, columns });
        }

        self.configs.save();
    }

    pub fn dimensions(&self) -> (u32, u32) {
        let width = self.columns() as u32 * self.tile_width
            + (self.columns() as u32 + 1) * self.grid_margins as u32;

        let height = self.rows() as u32 * self.tile_height
            + (self.rows() as u32 + 1) * self.grid_margins as u32;

        (width, height)
    }

    fn zone_area(&self, row: usize, column: usize) -> Rect {
        let work_area = unsafe { get_work_area() };

        let zone_width = (work_area.width
            - self.border_margins as i32 * 2
            - (self.columns() - 1) as i32 * self.zone_margins as i32)
            / self.columns() as i32;
        let zone_height = (work_area.height
            - self.border_margins as i32 * 2
            - (self.rows() - 1) as i32 * self.zone_margins as i32)
            / self.rows() as i32;

        let x = column as i32 * zone_width
            + self.border_margins as i32
            + column as i32 * self.zone_margins as i32
            + work_area.x;
        let y = row as i32 * zone_height
            + self.border_margins as i32
            + row as i32 * self.zone_margins as i32
            + work_area.y;

        Rect {
            x,
            y,
            width: zone_width,
            height: zone_height,
        }
    }

    fn rows(&self) -> usize {
        self.tiles.len()
    }

    fn columns(&self) -> usize {
        self.tiles[0].len()
    }

    pub fn add_row(&mut self) {
        let tile = Tile {
            normal_color: self.tile_normal_color,
            hovered_color: self.tile_hovered_color,
            selected_color: self.tile_selected_color,
            frame_color: self.tile_frame_color,
            ..Default::default()
        };
        self.tiles.push(vec![tile; self.columns()]);
        self.save_config();
    }

    pub fn add_column(&mut self) {
        for row in self.tiles.iter_mut() {
            let tile = Tile {
                normal_color: self.tile_normal_color,
                hovered_color: self.tile_hovered_color,
                selected_color: self.tile_selected_color,
                frame_color: self.tile_frame_color,
                ..Default::default()
            };
            row.push(tile);
        }
        self.save_config();
    }

    pub fn remove_row(&mut self) {
        if self.rows() > 1 {
            self.tiles.pop();
        }
        self.save_config();
    }

    pub fn remove_column(&mut self) {
        if self.columns() > 1 {
            for row in self.tiles.iter_mut() {
                row.pop();
            }
        }
        self.save_config();
    }

    fn tile_area(&self, row: usize, column: usize) -> Rect {
        let x =
            column as i32 * self.tile_width as i32 + (column as i32 + 1) * self.grid_margins as i32;

        let y = row as i32 * self.tile_height as i32 + (row as i32 + 1) * self.grid_margins as i32;

        Rect {
            x,
            y,
            width: self.tile_width as i32,
            height: self.tile_height as i32,
        }
    }

    pub fn reposition(&mut self) {
        let work_area = unsafe { get_work_area() };
        let dimensions = self.dimensions();

        let rect = Rect {
            x: work_area.width / 2 - dimensions.0 as i32 / 2 + work_area.x,
            y: work_area.height / 2 - dimensions.1 as i32 / 2 + work_area.y,
            width: dimensions.0 as i32,
            height: dimensions.1 as i32,
        };

        self.grid_window.as_mut().unwrap().set_pos(rect, None);
    }

    /// Returns true if a change in highlighting occured
    pub unsafe fn highlight_tiles(&mut self, point: (i32, i32)) -> Option<Rect> {
        let original_tiles = self.tiles.clone();
        let mut hovered_rect = None;

        for row in 0..self.rows() {
            for column in 0..self.columns() {
                let tile_area = self.tile_area(row, column);

                if tile_area.contains_point(point) {
                    self.tiles[row][column].hovered = true;

                    self.hovered_tile = Some((row, column));
                    hovered_rect = Some(self.zone_area(row, column));
                } else {
                    self.tiles[row][column].hovered = false;
                }
            }
        }

        if let Some(rect) = self.shift_hover_and_calc_rect(true) {
            hovered_rect = Some(rect);
        }

        if original_tiles == self.tiles {
            None
        } else {
            hovered_rect
        }
    }

    unsafe fn shift_hover_and_calc_rect(&mut self, highlight: bool) -> Option<Rect> {
        if self.shift_down || self.cursor_down {
            if let Some(selected_tile) = self.selected_tile {
                if let Some(hovered_tile) = self.hovered_tile {
                    let selected_zone = self.zone_area(selected_tile.0, selected_tile.1);
                    let hovered_zone = self.zone_area(hovered_tile.0, hovered_tile.1);

                    let from_tile;
                    let to_tile;

                    let hovered_rect = if hovered_zone.x < selected_zone.x
                        && hovered_zone.y > selected_zone.y
                    {
                        from_tile = (selected_tile.0, hovered_tile.1);
                        to_tile = (hovered_tile.0, selected_tile.1);

                        let from_zone = self.zone_area(from_tile.0, from_tile.1);
                        let to_zone = self.zone_area(to_tile.0, to_tile.1);

                        Rect {
                            x: from_zone.x,
                            y: from_zone.y,
                            width: (to_zone.x + to_zone.width) - from_zone.x,
                            height: (to_zone.y + to_zone.height) - from_zone.y,
                        }
                    } else if hovered_zone.y < selected_zone.y && hovered_zone.x > selected_zone.x {
                        from_tile = (hovered_tile.0, selected_tile.1);
                        to_tile = (selected_tile.0, hovered_tile.1);

                        let from_zone = self.zone_area(from_tile.0, from_tile.1);
                        let to_zone = self.zone_area(to_tile.0, to_tile.1);

                        Rect {
                            x: from_zone.x,
                            y: from_zone.y,
                            width: (to_zone.x + to_zone.width) - from_zone.x,
                            height: (to_zone.y + to_zone.height) - from_zone.y,
                        }
                    } else if hovered_zone.x > selected_zone.x || hovered_zone.y > selected_zone.y {
                        from_tile = selected_tile;
                        to_tile = hovered_tile;

                        Rect {
                            x: selected_zone.x,
                            y: selected_zone.y,
                            width: (hovered_zone.x + hovered_zone.width) - selected_zone.x,
                            height: (hovered_zone.y + hovered_zone.height) - selected_zone.y,
                        }
                    } else {
                        from_tile = hovered_tile;
                        to_tile = selected_tile;

                        Rect {
                            x: hovered_zone.x,
                            y: hovered_zone.y,
                            width: (selected_zone.x + selected_zone.width) - hovered_zone.x,
                            height: (selected_zone.y + selected_zone.height) - hovered_zone.y,
                        }
                    };

                    if highlight {
                        for row in from_tile.0..=to_tile.0 {
                            for column in from_tile.1..=to_tile.1 {
                                self.tiles[row][column].hovered = true;
                            }
                        }
                    }

                    return Some(hovered_rect);
                }
            }
        }

        None
    }

    pub unsafe fn select_tile(&mut self, point: (i32, i32)) -> bool {
        if self.cursor_down || self.shift_down {
            return false;
        }

        let previously_selected = self.selected_tile;

        for row in 0..self.rows() {
            for column in 0..self.columns() {
                let tile_area = self.tile_area(row, column);

                if tile_area.contains_point(point) {
                    self.tiles[row][column].selected = true;

                    self.selected_tile = Some((row, column));
                } else {
                    self.tiles[row][column].selected = false;
                }
            }
        }

        self.selected_tile != previously_selected
    }

    pub fn get_max_area(&self) -> Rect {
        let from_zone = self.zone_area(0, 0);
        let to_zone = self.zone_area(self.rows() - 1, self.columns() - 1);

        Rect {
            x: from_zone.x,
            y: from_zone.y,
            width: (to_zone.x + to_zone.width) - from_zone.x,
            height: (to_zone.y + to_zone.height) - from_zone.y,
        }
    }

    pub unsafe fn selected_area(&mut self) -> Option<Rect> {
        if let Some(shift_rect) = self.shift_hover_and_calc_rect(false) {
            return Some(shift_rect);
        }

        self.selected_tile
            .map(|tile| self.zone_area(tile.0, tile.1))
    }

    pub fn unhighlight_all_tiles(&mut self) {
        self.tiles
            .iter_mut()
            .for_each(|row| row.iter_mut().for_each(|tile| tile.hovered = false));
    }

    pub fn unselect_all_tiles(&mut self) {
        self.tiles
            .iter_mut()
            .for_each(|row| row.iter_mut().for_each(|tile| tile.selected = false));
    }

    pub unsafe fn draw(&self, window: Window) {
        let mut paint: PAINTSTRUCT = mem::zeroed();
        //paint.fErase = 1;

        let hdc = BeginPaint(window.0, &mut paint);

        for row in 0..self.rows() {
            for column in 0..self.columns() {
                self.tiles[row][column].draw(hdc, self.tile_area(row, column));
            }
        }

        let _ = EndPaint(window.0, &paint);
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
struct Tile {
    selected: bool,
    hovered: bool,
    frame_color: COLORREF,
    normal_color: COLORREF,
    hovered_color: COLORREF,
    selected_color: COLORREF,
}

impl Tile {
    unsafe fn draw(self, hdc: HDC, area: Rect) {
        let fill_brush = self.fill_brush();
        let frame_brush = CreateSolidBrush(self.frame_color);

        FillRect(hdc, &area.into(), fill_brush);
        FrameRect(hdc, &area.into(), frame_brush);

        let _ = DeleteObject(fill_brush);
        let _ = DeleteObject(frame_brush);
    }

    unsafe fn fill_brush(self) -> HBRUSH {
        let color = if self.selected {
            self.selected_color
        } else if self.hovered {
            self.hovered_color
        } else {
            self.normal_color
        };

        CreateSolidBrush(color)
    }
}
