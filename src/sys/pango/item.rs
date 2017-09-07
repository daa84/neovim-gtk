use std::ptr;
use std::mem;

use pango_sys;

use glib_ffi;
use gobject_ffi;
use glib::translate::*;

use super::analysis;

glib_wrapper! {
    pub struct Item(Boxed<pango_sys::PangoItem>);

    match fn {
        copy => |ptr| pango_sys::pango_item_copy(ptr as *mut pango_sys::PangoItem),
        free => |ptr| pango_sys::pango_item_free(ptr),
        get_type => || pango_sys::pango_item_get_type(),
    }
}

impl Item {
    #[cfg(test)]
    pub fn new() -> Self {
        unsafe {
            from_glib_none(pango_sys::pango_item_new())
        }
    }

    #[cfg(test)]
    pub fn set_offset(&mut self, offset: i32, length: i32, num_chars: i32) {
        self.0.offset = offset;
        self.0.length = length;
        self.0.num_chars = num_chars;
    }

    pub fn analysis(&self) -> analysis::Analysis {
        analysis::Analysis::from(&self.0.analysis)
    }

    pub fn offset(&self) -> (usize, usize, usize) {
        (self.0.offset as usize, self.0.length as usize, self.0.num_chars as usize)
    }

    pub fn length(&self) -> i32 {
        self.0.length
    }
}
