use pango;
use cairo;

use pango_cairo_sys as ffi;

use glib::translate::*;

pub fn show_glyph_string(cr: &cairo::Context, font: &pango::Font, glyphs: &pango::GlyphString) {
    unsafe {
        ffi::pango_cairo_show_glyph_string(
            mut_override(cr.to_glib_none().0),
            font.to_glib_none().0,
            mut_override(glyphs.to_glib_none().0),
        );
    }
}
