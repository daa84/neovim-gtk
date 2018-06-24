mod context;
mod itemize;
mod model_clip_iterator;

pub use self::context::{Context, FontFeatures};
pub use self::context::CellMetrics;
use self::model_clip_iterator::{ModelClipIteratorFactory, RowView};

use color;
use sys::pango::*;
use sys::pangocairo::*;
use pango;
use cairo;
use pangocairo;

use cursor::Cursor;
use ui_model;

pub fn clear(ctx: &cairo::Context) {
    ctx.set_operator(cairo::Operator::Clear);
    ctx.paint();
}

pub fn fill_background(ctx: &cairo::Context, color_model: &color::ColorModel) {
    // must be dest over here
    //ctx.set_operator(cairo::Operator::DestOver);
    ctx.set_source_rgba(
        color_model.bg_color.0,
        color_model.bg_color.1,
        color_model.bg_color.2,
        0.5,
    );
    ctx.paint();
}

pub fn render<C: Cursor>(
    ctx: &cairo::Context,
    cursor: &C,
    font_ctx: &context::Context,
    ui_model: &ui_model::UiModel,
    color_model: &color::ColorModel,
) {
    let cell_metrics = font_ctx.cell_metrics();
    let &CellMetrics { char_width, .. } = cell_metrics;
    let (cursor_row, cursor_col) = ui_model.get_cursor();

    // draw text
    ctx.set_operator(cairo::Operator::Over);

    for cell_view in ui_model.get_clip_iterator(ctx, cell_metrics) {
        let mut line_x = 0.0;

        for (col, cell) in cell_view.line.line.iter().enumerate() {
            draw_cell(&cell_view, color_model, cell, col, line_x);
            draw_underline(&cell_view, color_model, cell, line_x);

            line_x += char_width;
        }
    }


    // draw cursor
    ctx.set_operator(cairo::Operator::Xor);
    let (_x1, _y1, x2, y2) = ctx.clip_extents();
    let line_x = cursor_col as f64 * cell_metrics.char_width;
    let line_y = cursor_row as f64 * cell_metrics.line_height;

    if line_x < x2 && line_y < y2 {
        if let Some(cursor_line) = ui_model.model().get(cursor_row) {
            let double_width = cursor_line
                .line
                .get(cursor_col + 1)
                .map_or(false, |c| c.attrs.double_width);
            ctx.move_to(line_x, line_y);
            cursor.draw(ctx, font_ctx, line_y, double_width, &color_model);
        }
    }

    // draw background
    ctx.set_operator(cairo::Operator::DestOver);
    for cell_view in ui_model.get_clip_iterator(ctx, cell_metrics) {
        let mut line_x = 0.0;

        for (col, cell) in cell_view.line.line.iter().enumerate() {
            draw_cell_bg(&cell_view, color_model, cell, col, line_x);
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
            cell_metrics:
                &CellMetrics {
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

            pangocairo::functions::show_error_underline(
                ctx,
                line_x,
                undercurl_y,
                char_width,
                undercurl_height,
            );
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

fn draw_cell_bg(
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
        cell_metrics:
            &CellMetrics {
                char_width,
                line_height,
                ..
            },
        ..
    } = cell_view;

    let bg = color_model.cell_bg(cell);

    if let Some(bg) = bg {
        if !line.is_binded_to_item(col) {
            if bg != &color_model.bg_color {
                ctx.set_source_rgb(bg.0, bg.1, bg.2);
                ctx.rectangle(line_x, line_y, char_width, line_height);
                ctx.fill();
            }
        } else {
            ctx.set_source_rgb(bg.0, bg.1, bg.2);
            ctx.rectangle(
                line_x,
                line_y,
                char_width * line.item_len_from_idx(col) as f64,
                line_height,
            );
            ctx.fill();
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
        cell_metrics: &CellMetrics { ascent, .. },
        ..
    } = cell_view;

    if let Some(item) = line.item_line[col].as_ref() {
        if let Some(ref glyphs) = item.glyphs {
            let fg = color_model.actual_cell_fg(cell);

            ctx.move_to(line_x, line_y + ascent);
            ctx.set_source_rgb(fg.0, fg.1, fg.2);

            show_glyph_string(ctx, item.font(), glyphs);
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
            let styled_line = ui_model::StyledLine::from(line, color_model, ctx.font_features());
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
