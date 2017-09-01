mod context;

pub use self::context::Context;

use color;
use sys::pango::*;
use pango;
use cairo;
use pangocairo::CairoContextExt;
use ui_model;

pub fn render(
    ctx: &cairo::Context,
    ui_model: &ui_model::UiModel,
    color_model: &color::ColorModel,
    line_height: f64,
    char_width: f64,
) {
    ctx.set_source_rgb(
        color_model.bg_color.0,
        color_model.bg_color.1,
        color_model.bg_color.2,
    );
    ctx.paint();

    let mut line_y = line_height;

    for line in ui_model.model() {
        let mut line_x = 0.0;

        for i in 0..line.line.len() {
            ctx.move_to(line_x, line_y);
            if let Some(item) = line.item_line[i].as_ref() {
                if let Some(ref glyphs) = item.glyphs {
                    let (_, fg) = color_model.cell_colors(&line.line[i]);
                    ctx.set_source_rgb(fg.0, fg.1, fg.2);
                    ctx.show_glyph_string(item.font(), glyphs);
                }
            }
            line_x += char_width;
        }
        line_y += line_height;
    }
}

pub fn shape_dirty(ctx: &context::Context, ui_model: &mut ui_model::UiModel) {
    for line in ui_model.model_mut() {
        if line.dirty_line {
            let styled_line = ui_model::StyledLine::from(line);
            let items = ctx.itemize(&styled_line);
            line.merge(&styled_line, &items);

            for i in 0..line.line.len() {
                if line[i].dirty {
                    if let Some(mut item) = line.get_item_mut(i) {
                        let mut glyphs = pango::GlyphString::new();
                        {
                            let analysis = item.analysis();
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
