use sys::pango::*;
use pango;
use cairo;
use pangocairo::CairoContextExt;
use std::ffi::CString;

pub fn render(ctx: &cairo::Context) {
   let pango_context = ctx.create_pango_context();
   let text = "TEST String".to_owned().into_bytes();
   let len = text.len();
   let text = CString::new(text).unwrap();
   let attr_list = pango::AttrList::new();

   let items = pango_itemize(&pango_context, &text, 0, len, &attr_list);
   for item in items {
       let mut glyphs = pango::GlyphString::new();
       let analysis = item.analysis();
       pango_shape(&text, len, &analysis, &mut glyphs);
       let (ink, logical) = glyphs.extents(&analysis.font());
   }
}
