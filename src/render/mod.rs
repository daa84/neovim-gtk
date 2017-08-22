use sys::pango::*;
use pango;
use pango::prelude::*;
use cairo;
use pangocairo::CairoContextExt;
use std::ffi::CString;

pub fn render(ctx: &cairo::Context, font_desc: pango::FontDescription) {
   let pango_context = ctx.create_pango_context();
   pango_context.set_font_description(&font_desc);

   let text = "TEST String".to_owned().into_bytes();
   let len = text.len();
   let text = CString::new(text).unwrap();
   let attr_list = pango::AttrList::new();

   ctx.move_to(0.0, 50.0);
   let items = pango_itemize(&pango_context, &text, 0, len, &attr_list);
   for item in items {
       let mut glyphs = pango::GlyphString::new();
       let analysis = item.analysis();
       pango_shape(&text, len, &analysis, &mut glyphs);
       let font = analysis.font();
       let (ink, logical) = glyphs.extents(&font);
       ctx.show_glyph_string(&font, &glyphs);
   }
}
