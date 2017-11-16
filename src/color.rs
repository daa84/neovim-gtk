use std;
use gdk;
use ui_model::Cell;
use theme::Theme;

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

    pub fn to_hex(&self) -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            (self.0 * 255.0) as u8,
            (self.1 * 255.0) as u8,
            (self.2 * 255.0) as u8
        )
    }
}

pub struct ColorModel {
    pub bg_color: Color,
    pub fg_color: Color,
    pub sp_color: Color,
    pub theme: Theme,
}

impl ColorModel {
    pub fn new() -> Self {
        ColorModel {
            bg_color: COLOR_BLACK,
            fg_color: COLOR_WHITE,
            sp_color: COLOR_RED,
            theme: Theme::new(),
        }
    }

    pub fn cell_colors<'a>(&'a self, cell: &'a Cell) -> (Option<&'a Color>, &'a Color) {
        if !cell.attrs.reverse {
            (
                cell.attrs.background.as_ref(),
                cell.attrs.foreground.as_ref().unwrap_or(&self.fg_color),
            )
        } else {
            (
                cell.attrs.foreground.as_ref().or(Some(&self.fg_color)),
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

    pub fn actual_cell_fg<'a>(&'a self, cell: &'a Cell) -> &'a Color {
        if !cell.attrs.reverse {
            cell.attrs.foreground.as_ref().unwrap_or(&self.fg_color)
        } else {
            cell.attrs.background.as_ref().unwrap_or(&self.bg_color)
        }
    }

    pub fn cell_bg<'a>(&'a self, cell: &'a Cell) -> Option<&'a Color> {
        if !cell.attrs.reverse {
            cell.attrs.background.as_ref()
        } else {
            cell.attrs.foreground.as_ref().or(Some(&self.fg_color))
        }
    }

    pub fn actual_cell_bg<'a>(&'a self, cell: &'a Cell) -> &'a Color {
        if !cell.attrs.reverse {
            cell.attrs.background.as_ref().unwrap_or(&self.bg_color)
        } else {
            cell.attrs.foreground.as_ref().unwrap_or(&self.fg_color)
        }
    }

    #[inline]
    pub fn actual_cell_sp<'a>(&'a self, cell: &'a Cell) -> &'a Color {
        cell.attrs.special.as_ref().unwrap_or(&self.sp_color)
    }

    pub fn pmenu_bg(&self) -> &Color {
        if let Some(ref pmenu) = self.theme.pmenu {
            pmenu.bg.as_ref().unwrap_or(&self.bg_color)
        } else {
            &self.bg_color
        }
    }


    pub fn pmenu_fg(&self) -> &Color {
        if let Some(ref pmenu) = self.theme.pmenu {
            pmenu.fg.as_ref().unwrap_or(&self.fg_color)
        } else {
            &self.fg_color
        }
    }


    pub fn pmenu_bg_sel(&self) -> &Color {
        if let Some(ref pmenu) = self.theme.pmenu {
            pmenu.bg_sel.as_ref().unwrap_or(&self.bg_color)
        } else {
            &self.bg_color
        }
    }


    pub fn pmenu_fg_sel(&self) -> &Color {
        if let Some(ref pmenu) = self.theme.pmenu {
            pmenu.fg_sel.as_ref().unwrap_or(&self.fg_color)
        } else {
            &self.fg_color
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_hex() {
        let col = Color(0.0, 1.0, 0.0);
        assert_eq!("#00FF00", &col.to_hex());
    }
}
