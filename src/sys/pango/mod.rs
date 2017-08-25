pub mod item;
mod analysis;

use std::ptr;

use pango;
use pango_sys;

use glib::translate::*;

pub fn pango_itemize(
    context: &pango::Context,
    text: &String,
    attrs: &pango::AttrList
) -> Vec<item::Item> {
    unsafe {
        FromGlibPtrContainer::from_glib_container(pango_sys::pango_itemize(
            context.to_glib_none().0,
            text.as_ptr() as *const i8,
            0,
            text.len() as i32,
            attrs.to_glib_none().0,
            ptr::null_mut(),
        ))
    }
}

pub fn pango_shape(
    text: &String,
    offset: usize,
    length: usize,
    analysis: &analysis::Analysis,
    glyphs: &mut pango::GlyphString,
) {
    debug_assert!(offset + length <= text.len());

    unsafe {
        pango_sys::pango_shape(
            (text.as_ptr() as *const i8).offset(offset as isize),
            length as i32,
            analysis.to_glib_ptr(),
            glyphs.to_glib_none_mut().0,
        );
    }
}

