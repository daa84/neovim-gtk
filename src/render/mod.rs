mod context;
mod itemize;
mod model_clip_iterator;

pub use self::context::Context;
pub use self::context::CellMetrics;
use self::model_clip_iterator::{RowView, ModelClipIteratorFactory};

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

    let cell_metrics = font_ctx.cell_metrics();
    let &CellMetrics { char_width, .. } = cell_metrics;
    let (cursor_row, cursor_col) = ui_model.get_cursor();

    for cell_view in ui_model.get_clip_iterator(ctx, cell_metrics) {
        let mut line_x = 0.0;
        let RowView { line, row, line_y, .. } = cell_view;

        for (col, cell) in line.line.iter().enumerate() {

            draw_cell(&cell_view, color_model, cell, col, line_x);

            draw_underline(&cell_view, color_model, cell, line_x);


            if row == cursor_row && col == cursor_col {
                let double_width = line.line.get(col + 1).map_or(
                    false,
                    |c| c.attrs.double_width,
                );
                ctx.move_to(line_x, line_y);
                cursor.draw(
                    ctx,
                    font_ctx,
                    mode,
                    line_y,
                    double_width,
                    color_model.actual_cell_bg(cell),
                );
            }

            line_x += char_width;
        }
    }
}

fn draw_underline(
    cell_view: &RowView,
    color_model: &color::ColorModel,
    cell: &ui_model::Cell,
    line_x: f64,
) {

    if cell.attrs.underline || cell.attrs.undercurl {

        let &RowView {
            ctx,
            line_y,
            cell_metrics: &CellMetrics {
                line_height,
                char_width,
                underline_position,
                underline_thickness,
                ..
            },
            ..
        } = cell_view;

        if cell.attrs.undercurl {
            let sp = color_model.actual_cell_sp(cell);
            ctx.set_source_rgba(sp.0, sp.1, sp.2, 0.7);

            let max_undercurl_height = (line_height - underline_position) * 2.0;
            let undercurl_height = (underline_thickness * 4.0).min(max_undercurl_height);
            let undercurl_y = line_y + underline_position - undercurl_height / 2.0;

            ctx.show_error_underline(line_x, undercurl_y, char_width, undercurl_height);
        } else if cell.attrs.underline {
            let fg = color_model.actual_cell_fg(cell);
            ctx.set_source_rgb(fg.0, fg.1, fg.2);
            ctx.set_line_width(underline_thickness);
            ctx.move_to(line_x, line_y + underline_position);
            ctx.line_to(line_x + char_width, line_y + underline_position);
            ctx.stroke();
        }
    }
}

fn draw_cell(
    cell_view: &RowView,
    color_model: &color::ColorModel,
    cell: &ui_model::Cell,
    col: usize,
    line_x: f64,
) {

    let &RowView {
        ctx,
        line,
        line_y,
        cell_metrics: &CellMetrics {
            char_width,
            line_height,
            ascent,
            ..
        },
        ..
    } = cell_view;

    let (bg, fg) = color_model.cell_colors(cell);

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
            if bg != &color_model.bg_color {
                ctx.set_source_rgb(bg.0, bg.1, bg.2);
                ctx.rectangle(line_x, line_y, char_width, line_height);
                ctx.fill();
            }
        }
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

            for (col, cell) in line.line.iter_mut().enumerate() {
                if cell.dirty {
                    if let Some(item) = line.item_line[col].as_mut() {
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

                cell.dirty = false;
            }

            line.dirty_line = false;
        }
    }
}
