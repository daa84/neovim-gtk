use std::ffi::CString;

use pangocairo::FontMap;
use pango::prelude::*;
use pango;

use sys::pango as sys_pango;

use ui_model::StyledLine;

pub struct Context {
    pango_context: pango::Context,
}

impl Context {
    pub fn new(font_desc: &pango::FontDescription) -> Self {
        Context { pango_context: create_pango_context(font_desc) }
    }

    pub fn update(&mut self, font_desc: &pango::FontDescription) {
        self.pango_context = create_pango_context(font_desc);
    }

    pub fn itemize(&self, line: &StyledLine) -> Vec<sys_pango::Item> {
        sys_pango::pango_itemize(&self.pango_context, &line.line_str, &line.attr_list)
    }
}

fn create_pango_context(font_desc: &pango::FontDescription) -> pango::Context {
    let font_map = FontMap::get_default();
    let pango_context = font_map.create_context().unwrap();
    pango_context.set_font_description(&font_desc);

    pango_context
}
