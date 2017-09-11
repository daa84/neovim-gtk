use std::ptr;
use std::mem;

use pango_sys;
use pango;

use glib_ffi;
use glib::translate::*;

glib_wrapper! {
    pub struct AttrIterator(Boxed<pango_sys::PangoAttrIterator>);

    match fn {
        copy => |ptr| pango_sys::pango_attr_iterator_copy(ptr as *mut _),
        free => |ptr| pango_sys::pango_attr_iterator_destroy(ptr),
    }
}

pub trait AttrIteratorFactory {
    fn get_iterator(&self) -> AttrIterator;
}

impl AttrIteratorFactory for pango::AttrList {
    fn get_iterator(&self) -> AttrIterator {
        unsafe {
            from_glib_none(pango_sys::pango_attr_list_get_iterator(
                self.to_glib_none().0,
            ))
        }
    }
}
