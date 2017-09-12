mod context;
mod itemize;

pub use self::context::Context;
pub use self::context::CellMetrics;

use mode;
use color;
use sys::pango::*;
use pango;
use cairo;
use cursor;
use pangocairo::CairoContextExt;
use ui_model;

pub fn render(
    ctx: &cairo::Context,
    cursor: &cursor::Cursor,
    font_ctx: &context::Context,
    ui_model: &ui_model::UiModel,
    color_model: &color::ColorModel,
    mode: &mode::Mode,
) {
    ctx.set_source_rgb(
        color_model.bg_color.0,
        color_model.bg_color.1,
        color_model.bg_color.2,
    );
    ctx.paint();

    let &CellMetrics {
        line_height,
        char_width,
        underline_position,
        underline_thickness,
        ascent,
        ..
    } = font_ctx.cell_metrics();
    let mut line_y = 0.0;
    let (cursor_row, cursor_col) = ui_model.get_cursor();

    for (row, line) in ui_model.model().iter().enumerate() {
        let mut line_x = 0.0;

        for col in 0..line.line.len() {
            let cell = &line.line[col];

            let (bg, fg) = color_model.cell_colors(cell);

            // draw cell
            if let Some(item) = line.item_line[col].as_ref() {

                if let Some(bg) = bg {
                    ctx.set_source_rgb(bg.0, bg.1, bg.2);
                    ctx.rectangle(
                        line_x,
                        line_y,
                        char_width * line.item_len_from_idx(col) as f64,
                        line_height,
                    );
                    ctx.fill();
                }

                if let Some(ref glyphs) = item.glyphs {
                    ctx.move_to(line_x, line_y + ascent);
                    ctx.set_source_rgb(fg.0, fg.1, fg.2);
                    ctx.show_glyph_string(item.font(), glyphs);
                }

            } else if !line.is_binded_to_item(col) {
                let bg = color_model.cell_bg(cell);
                if let Some(bg) = bg {
                    ctx.set_source_rgb(bg.0, bg.1, bg.2);
                    ctx.rectangle(line_x, line_y, char_width, line_height);
                    ctx.fill();
                }
            }

            if cell.attrs.underline || cell.attrs.undercurl {
                if cell.attrs.undercurl {
                    //FIXME: don't repaint all lines on changes
                    let sp = color_model.actual_cell_sp(cell);
                    ctx.set_source_rgba(sp.0, sp.1, sp.2, 0.7);

                    let max_undercurl_height = (line_height - underline_position) * 2.0;
                    let undercurl_height = (underline_thickness * 4.0).min(max_undercurl_height);
                    let undercurl_y = line_y + underline_position - undercurl_height / 2.0;

                    ctx.show_error_underline(
                        line_x,
                        undercurl_y,
                        char_width,
                        undercurl_height
                    );
                } else if cell.attrs.underline {
                    ctx.set_source_rgb(fg.0, fg.1, fg.2);
                    ctx.set_line_width(underline_thickness);
                    ctx.move_to(line_x, line_y + underline_position);
                    ctx.line_to(line_x + char_width, line_y + underline_position);
                    ctx.stroke();
                }
            }

            if row == cursor_row && col == cursor_col {
                ctx.move_to(line_x, line_y);
                cursor.draw(
                    ctx,
                    font_ctx,
                    mode,
                    line_y,
                    false, //TODO: double_width,
                    color_model.actual_cell_bg(cell),
                );
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
                    if let Some(item) = line.get_item_mut(i) {
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

                        item.set_glyphs(ctx, glyphs);
                    }
                }

                line[i].dirty = false;
            }

            line.dirty_line = false;
        }
    }
}
