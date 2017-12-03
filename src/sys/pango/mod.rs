mod item;
mod analysis;
mod attr_iterator;

pub use self::item::Item;
pub use self::analysis::Analysis;
pub use self::attr_iterator::{AttrIterator, AttrIteratorFactory};

use std::ptr;

use pango;
use pango_sys;
use glib_ffi;

use glib::translate::*;

pub fn pango_itemize(
    context: &pango::Context,
    text: &str,
    start_index: usize,
    length: usize,
    attrs: &pango::AttrList,
    cached_iter: Option<&mut AttrIterator>,
) -> Vec<Item> {
    unsafe {
        //FromGlibPtrContainer::from_glib_full(pango_sys::pango_itemize(
        from_glib_full_as_vec(pango_sys::pango_itemize(
            context.to_glib_none().0,
            text.as_ptr() as *const i8,
            start_index as i32,
            length as i32,
            attrs.to_glib_none().0,
            cached_iter.map(|iter| iter.to_glib_none_mut().0).unwrap_or(ptr::null_mut()),
        ))
    }
}


unsafe fn from_glib_full_as_vec(ptr: *mut glib_ffi::GList) -> Vec<Item> {
    let num = glib_ffi::g_list_length(ptr) as usize;
    FromGlibContainer::from_glib_full_num(ptr, num)
}

pub fn pango_shape(
    text: &str,
    offset: usize,
    length: usize,
    analysis: &Analysis,
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

