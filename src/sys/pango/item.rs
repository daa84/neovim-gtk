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
    pub fn analysis(&self) -> analysis::Analysis {
        analysis::Analysis::from(&self.0.analysis)
    }

    pub fn offset(&self) -> (usize, usize, usize) {
        (self.0.offset as usize, self.0.length as usize, self.0.num_chars as usize)
    }
}
