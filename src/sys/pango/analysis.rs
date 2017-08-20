use pango_sys;
use pango;

use glib::translate::*;

pub struct Analysis<'a>(&'a pango_sys::PangoAnalysis);

impl <'a> Analysis <'a> {
    pub fn from(analysis: &'a pango_sys::PangoAnalysis) -> Self {
        Analysis(analysis)
    }

    pub fn font(&self) -> pango::Font {
        unsafe {
            from_glib_none(self.0.font)
        }
    }

    pub fn to_glib_ptr(&self) -> *const pango_sys::PangoAnalysis {
        self.0
    }
}
