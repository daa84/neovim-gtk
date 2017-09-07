mod context;

pub use self::context::Context;
pub use self::context::CellMetrics;

use color;
use sys::pango::*;
use pango;
use cairo;
use pangocairo::CairoContextExt;
use ui_model;

pub fn render(
    ctx: &cairo::Context,
    font_ctx: &context::Context,
    ui_model: &ui_model::UiModel,
    color_model: &color::ColorModel,
) {
    // TODO: underline
    // TODO: undercurl
    // TODO: cursor
    ctx.set_source_rgb(
        color_model.bg_color.0,
        color_model.bg_color.1,
        color_model.bg_color.2,
    );
    ctx.paint();

    let &CellMetrics {
        line_height,
        char_width,
        ..
    } = font_ctx.cell_metrics();
    let mut line_y = 0.0;
    let ascent = font_ctx.ascent();

    for line in ui_model.model() {
        let mut line_x = 0.0;

        for i in 0..line.line.len() {

            if let Some(item) = line.item_line[i].as_ref() {
                let (bg, fg) = color_model.cell_colors(&line.line[i]);

                if let Some(bg) = bg {
                    ctx.set_source_rgb(bg.0, bg.1, bg.2);
                    ctx.rectangle(
                        line_x,
                        line_y,
                        char_width * line.item_len_from_idx(i) as f64,
                        line_height,
                    );
                    ctx.fill();
                }

                if let Some(ref glyphs) = item.glyphs {
                    ctx.move_to(line_x, line_y + ascent);
                    ctx.set_source_rgb(fg.0, fg.1, fg.2);
                    ctx.show_glyph_string(item.font(), glyphs);
                }

            } else if !line.is_binded_to_item(i) {
                let bg = color_model.cell_bg(&line.line[i]);
                if let Some(bg) = bg {
                    ctx.set_source_rgb(bg.0, bg.1, bg.2);
                    ctx.rectangle(line_x, line_y, char_width, line_height);
                    ctx.fill();
                }
            }
            line_x += char_width;
        }
        line_y += line_height;
    }
}

pub fn shape_dirty(
    ctx: &context::Context,
    ui_model: &mut ui_model::UiModel,
    color_model: &color::ColorModel,
) {
    for line in ui_model.model_mut() {
        if line.dirty_line {
            let styled_line = ui_model::StyledLine::from(line, color_model);
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
