use std::collections::HashMap;

use neovim_lib::{Value, Neovim, NeovimApi};

use nvim::ErrorReport;
use color::Color;
use value::ValueMapExt;

pub struct Theme {
    pub pmenu: Option<Pmenu>,
}

impl Theme {
    pub fn new() -> Self {
        Theme { pmenu: None }
    }

    pub fn update(&mut self, nvim: &mut Neovim) {
        self.pmenu = Some(Pmenu::new(nvim));
    }
}

pub struct Pmenu {
    pub bg: Option<Color>,
    pub fg: Option<Color>,
    pub bg_sel: Option<Color>,
    pub fg_sel: Option<Color>,
}

impl Pmenu {
    pub fn new(nvim: &mut Neovim) -> Self {
        let (bg, fg) = get_hl_colors(nvim, "Pmenu");
        let (bg_sel, fg_sel) = get_hl_colors(nvim, "PmenuSel");

        Pmenu {
            bg,
            fg,
            bg_sel,
            fg_sel,
        }
    }
}

fn get_hl_color(map: &HashMap<&str, &Value>, color_name: &str) -> Option<Color> {
    if let Some(col) = map.get(color_name) {
        if let Some(col) = col.as_u64() {
            Some(Color::from_indexed_color(col))
        } else {
            None
        }
    } else {
        None
    }
}

fn get_hl_colors(nvim: &mut Neovim, hl: &str) -> (Option<Color>, Option<Color>) {
    nvim.get_hl_by_name(hl, true)
        .ok_and_report()
        .and_then(|m| if let Some(m) = m.to_attrs_map_report() {
            Some((
                get_hl_color(&m, "background"),
                get_hl_color(&m, "foreground"),
            ))
        } else {
            None
        })
        .unwrap_or((None, None))
}
