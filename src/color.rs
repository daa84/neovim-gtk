use std;
use gdk;
use ui_model::Cell;

#[derive(Clone, PartialEq, Debug)]
pub struct Color(pub f64, pub f64, pub f64);

pub const COLOR_BLACK: Color = Color(0.0, 0.0, 0.0);
pub const COLOR_WHITE: Color = Color(1.0, 1.0, 1.0);
pub const COLOR_RED: Color = Color(1.0, 0.0, 0.0);

impl<'a> Into<gdk::RGBA> for &'a Color {
    fn into(self) -> gdk::RGBA {
        gdk::RGBA {
            red: self.0,
            green: self.1,
            blue: self.2,
            alpha: 1.0,
        }
    }
}

impl Color {
    pub fn from_indexed_color(indexed_color: u64) -> Color {
        let r = ((indexed_color >> 16) & 0xff) as f64;
        let g = ((indexed_color >> 8) & 0xff) as f64;
        let b = (indexed_color & 0xff) as f64;
        Color(r / 255.0, g / 255.0, b / 255.0)
    }

    pub fn to_u16(&self) -> (u16, u16, u16) {
        (
            (std::u16::MAX as f64 * self.0) as u16,
            (std::u16::MAX as f64 * self.1) as u16,
            (std::u16::MAX as f64 * self.2) as u16,
        )
    }
}

pub struct ColorModel {
    pub bg_color: Color,
    pub fg_color: Color,
    pub sp_color: Color,
}

impl ColorModel {
    pub fn new() -> Self {
        ColorModel {
            bg_color: COLOR_BLACK,
            fg_color: COLOR_WHITE,
            sp_color: COLOR_RED,
        }
    }

    pub fn cell_colors<'a>(&'a self, cell: &'a Cell) -> (&'a Color, &'a Color) {
        if !cell.attrs.reverse {
            (
                cell.attrs.background.as_ref().unwrap_or(&self.bg_color),
                cell.attrs.foreground.as_ref().unwrap_or(&self.fg_color),
            )
        } else {
            (
                cell.attrs.foreground.as_ref().unwrap_or(&self.fg_color),
                cell.attrs.background.as_ref().unwrap_or(&self.bg_color),
            )
        }
    }

    pub fn cell_fg<'a>(&'a self, cell: &'a Cell) -> Option<&'a Color> {
        if !cell.attrs.reverse {
            cell.attrs.foreground.as_ref()
        } else {
            cell.attrs.background.as_ref().or(Some(&self.bg_color))
        }
    }
}
