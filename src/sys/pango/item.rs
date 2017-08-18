use std::ptr;
use std::mem;

use pango_sys;

use glib_ffi;
use glib::translate::*;

glib_wrapper! {
    pub struct Item(Boxed<pango_sys::PangoItem>);

    match fn {
        copy => |ptr| pango_sys::pango_item_copy(ptr as *mut pango_sys::PangoItem),
        free => |ptr| pango_sys::pango_item_free(ptr),
    }
}

impl Item {
    pub fn analysis(&self) -> &pango_sys::PangoAnalysis {
        &self.0.analysis
    }
}
