use pango_sys;
use pango;

use glib::translate::*;

pub fn new_features(features: &str) -> Option<pango::Attribute> {
    unsafe {
        from_glib_full(pango_sys::pango_attr_font_features_new(
                features.to_glib_none().0,
                ))
    }
}
