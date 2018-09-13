pub mod attribute;

use pango;
use pango_sys;

use glib::translate::*;

pub fn pango_shape(
    text: &str,
    offset: usize,
    length: usize,
    analysis: &pango::Analysis,
    glyphs: &mut pango::GlyphString,
) {
    debug_assert!(offset + length <= text.len());

    unsafe {
        pango_sys::pango_shape(
            (text.as_ptr() as *const i8).offset(offset as isize),
            length as i32,
            analysis.to_glib_none().0,
            glyphs.to_glib_none_mut().0,
        );
    }
}
