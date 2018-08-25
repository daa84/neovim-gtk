use std;

use gdk;

#[derive(Clone, PartialEq, Debug)]
pub struct Color(pub f64, pub f64, pub f64);

pub const COLOR_BLACK: Color = Color(0.0, 0.0, 0.0);
pub const COLOR_WHITE: Color = Color(1.0, 1.0, 1.0);
pub const COLOR_RED: Color = Color(1.0, 0.0, 0.0);

impl From<Color> for gdk::RGBA {
    fn from(color: Color) -> Self {
        gdk::RGBA {
            red: color.0,
            green: color.1,
            blue: color.2,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_hex() {
        let col = Color(0.0, 1.0, 0.0);
        assert_eq!("#00FF00", &col.to_hex());
    }
}
