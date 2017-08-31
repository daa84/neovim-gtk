use gdk;
use ui_model::Cell;

#[derive(Clone, PartialEq)]
pub struct Color(pub f64, pub f64, pub f64);

pub const COLOR_BLACK: Color = Color(0.0, 0.0, 0.0);
pub const COLOR_WHITE: Color = Color(1.0, 1.0, 1.0);
pub const COLOR_RED: Color = Color(1.0, 0.0, 0.0);

impl <'a> Into<gdk::RGBA> for &'a Color {
    fn into(self) -> gdk::RGBA {
        gdk::RGBA {
            red: self.0,
            green: self.1,
            blue: self.2,
            alpha: 1.0,
        }
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
        let bg = if let Some(ref bg) = cell.attrs.background {
            bg
        } else {
            &self.bg_color
        };
        let fg = if let Some(ref fg) = cell.attrs.foreground {
            fg
        } else {
            &self.fg_color
        };

        if cell.attrs.reverse {
            (fg, bg)
        } else {
            (bg, fg)
        }
    }
}
