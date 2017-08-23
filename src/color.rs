use gdk;

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
