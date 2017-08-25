mod context;

use sys::pango::*;
use pango;
use pango::prelude::*;
use cairo;
use pangocairo::{CairoContextExt, FontMap};
use std::ffi::CString;
use ui_model;

pub fn render(
    ctx: &cairo::Context,
    font_desc: pango::FontDescription,
    ui_model: &mut ui_model::UiModel,
) {
    let font_ctx = context::Context::new(&font_desc);

    shape_dirty(&font_ctx, ui_model);


    for line in ui_model.model_mut() {
        for i in 0..line.line.len() {
            let item = line.item_line[i].as_ref();
            if let Some(item) = item {
                if let Some(ref glyphs) = item.glyphs {
                    let analysis = item.item.analysis();
                    let font = analysis.font();
                    ctx.show_glyph_string(&font, glyphs);
                }
            }
        }
    }
}

fn shape_dirty(ctx: &context::Context, ui_model: &mut ui_model::UiModel) {
    for line in ui_model.model_mut() {
        if line.dirty_line {
            let styled_line = ui_model::StyledLine::from(line);
            let items = ctx.itemize(&styled_line);
            line.merge(&styled_line, &items);

            for i in 0..line.line.len() {
                if line[i].dirty {
                    let mut item = line.get_item_mut(i).unwrap();
                    let mut glyphs = pango::GlyphString::new();
                    {
                        let analysis = item.item.analysis();
                        let (offset, length, _) = item.item.offset();
                        pango_shape(
                            &styled_line.line_str,
                            offset,
                            length,
                            &analysis,
                            &mut glyphs,
                        );
                    }

                    item.set_glyphs(glyphs);
                }

                line[i].dirty = false;
            }

            line.dirty_line = false;
        }
    }
}

//pub fn render_test(ctx: &cairo::Context, font_desc: pango::FontDescription) {
//let font_map = FontMap::get_default();
//let pango_context = font_map.create_context().unwrap();
//pango_context.set_font_description(&font_desc);

//let text = "TEST String".to_owned();
//let attr_list = pango::AttrList::new();

//ctx.move_to(0.0, 50.0);
//let items = pango_itemize(&pango_context, &text, &attr_list);
//for item in items {
//let mut glyphs = pango::GlyphString::new();
//let analysis = item.analysis();
//pango_shape(&text, &analysis, &mut glyphs);
//let font = analysis.font();
//let (ink, logical) = glyphs.extents(&font);
//ctx.show_glyph_string(&font, &glyphs);
//}
//}
