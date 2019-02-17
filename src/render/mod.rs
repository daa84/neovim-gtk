mod context;
mod itemize;
mod model_clip_iterator;

pub use self::context::CellMetrics;
pub use self::context::{Context, FontFeatures};
use self::model_clip_iterator::{ModelClipIteratorFactory, RowView};

use cairo;
use color;
use pango;
use pangocairo;
use sys::pangocairo::*;

use cursor::{cursor_rect, Cursor};
use highlight::HighlightMap;
use ui_model;

trait ContextAlpha {
    fn set_source_rgbo(&self, &color::Color, Option<f64>);
}

impl ContextAlpha for cairo::Context {
    fn set_source_rgbo(&self, color: &color::Color, alpha: Option<f64>) {
        if let Some(alpha) = alpha {
            self.set_source_rgba(color.0, color.1, color.2, alpha);
        } else {
            self.set_source_rgb(color.0, color.1, color.2);
        }
    }
}

pub fn fill_background(ctx: &cairo::Context, hl: &HighlightMap, alpha: Option<f64>) {
    ctx.set_source_rgbo(&hl.bg_color, alpha);
    ctx.paint();
}

pub fn render<C: Cursor>(
    ctx: &cairo::Context,
    cursor: &C,
    font_ctx: &context::Context,
    ui_model: &ui_model::UiModel,
    hl: &HighlightMap,
    bg_alpha: Option<f64>,
) {
    let cell_metrics = font_ctx.cell_metrics();
    let &CellMetrics { char_width, .. } = cell_metrics;

    // draw background
    for row_view in ui_model.get_clip_iterator(ctx, cell_metrics) {
        let mut line_x = 0.0;

        for (col, cell) in row_view.line.line.iter().enumerate() {
            draw_cell_bg(&row_view, hl, cell, col, line_x, bg_alpha);
            line_x += char_width;
        }
    }

    // draw text
    for row_view in ui_model.get_clip_iterator(ctx, cell_metrics) {
        let mut line_x = 0.0;

        for (col, cell) in row_view.line.line.iter().enumerate() {
            draw_cell(&row_view, hl, cell, col, line_x, 0.0);
            draw_underline(&row_view, hl, cell, line_x, 0.0);

            line_x += char_width;
        }
    }

    draw_cursor(ctx, cursor, font_ctx, ui_model, hl, bg_alpha);
}

fn draw_cursor<C: Cursor>(
    ctx: &cairo::Context,
    cursor: &C,
    font_ctx: &context::Context,
    ui_model: &ui_model::UiModel,
    hl: &HighlightMap,
    bg_alpha: Option<f64>,
) {
    let cell_metrics = font_ctx.cell_metrics();
    let (cursor_row, cursor_col) = ui_model.get_cursor();

    let (x1, y1, x2, y2) = ctx.clip_extents();
    let line_x = cursor_col as f64 * cell_metrics.char_width;
    let line_y = cursor_row as f64 * cell_metrics.line_height;

    if line_x < x1 || line_y < y1 || line_x > x2 || line_y > y2 || !cursor.is_visible() {
        return;
    }

    let cell_metrics = font_ctx.cell_metrics();
    let row_view = ui_model.get_row_view(ctx, cell_metrics, cursor_row);
    let cell_start_col = row_view.line.cell_to_item(cursor_col);

    if let Some(cursor_line) = ui_model.model().get(cursor_row) {
        let double_width = cursor_line
            .line
            .get(cursor_col + 1)
            .map_or(false, |c| c.double_width);

        if cell_start_col >= 0 {
            let cell = &cursor_line[cursor_col];

            // clip cursor position
            let (clip_y, clip_width, clip_height) =
                cursor_rect(cursor.mode_info(), cell_metrics, line_y, double_width);
            ctx.rectangle(line_x, clip_y, clip_width, clip_height);
            ctx.clip();

            // repaint cell backgound
            ctx.set_operator(cairo::Operator::Source);
            fill_background(ctx, hl, bg_alpha);
            draw_cell_bg(&row_view, hl, cell, cursor_col, line_x, bg_alpha);

            // reapint cursor and text
            ctx.set_operator(cairo::Operator::Over);
            ctx.move_to(line_x, line_y);
            let cursor_alpha = cursor.draw(ctx, font_ctx, line_y, double_width, &hl);

            let cell_start_line_x =
                line_x - (cursor_col as i32 - cell_start_col) as f64 * cell_metrics.char_width;

            debug_assert!(cell_start_line_x >= 0.0);

            draw_cell(
                &row_view,
                hl,
                cell,
                cell_start_col as usize,
                cell_start_line_x,
                cursor_alpha,
            );
            draw_underline(&row_view, hl, cell, line_x, cursor_alpha);
        } else {
            ctx.move_to(line_x, line_y);
            cursor.draw(ctx, font_ctx, line_y, double_width, &hl);
        }
    }
}

fn draw_underline(
    cell_view: &RowView,
    hl: &HighlightMap,
    cell: &ui_model::Cell,
    line_x: f64,
    inverse_level: f64,
) {
    if cell.hl.underline || cell.hl.undercurl {
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

        if cell.hl.undercurl {
            let sp = hl.actual_cell_sp(cell).inverse(inverse_level);
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
        } else if cell.hl.underline {
            let fg = hl.actual_cell_fg(cell).inverse(inverse_level);
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
    hl: &HighlightMap,
    cell: &ui_model::Cell,
    col: usize,
    line_x: f64,
    bg_alpha: Option<f64>,
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

    let bg = hl.cell_bg(cell);

    if let Some(bg) = bg {
        if !line.is_binded_to_item(col) {
            if bg != &hl.bg_color {
                ctx.set_source_rgbo(bg, bg_alpha);
                ctx.rectangle(line_x, line_y, char_width, line_height);
                ctx.fill();
            }
        } else {
            ctx.set_source_rgbo(bg, bg_alpha);
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
    row_view: &RowView,
    hl: &HighlightMap,
    cell: &ui_model::Cell,
    col: usize,
    line_x: f64,
    inverse_level: f64,
) {
    let &RowView {
        ctx,
        line,
        line_y,
        cell_metrics: &CellMetrics { ascent, .. },
        ..
    } = row_view;

    if let Some(item) = line.item_line[col].as_ref() {
        if let Some(ref glyphs) = item.glyphs {
            let fg = hl.actual_cell_fg(cell).inverse(inverse_level);

            ctx.move_to(line_x, line_y + ascent);
            ctx.set_source_rgb(fg.0, fg.1, fg.2);

            show_glyph_string(ctx, item.font(), glyphs);
        }
    }
}

pub fn shape_dirty(ctx: &context::Context, ui_model: &mut ui_model::UiModel, hl: &HighlightMap) {
    for line in ui_model.model_mut() {
        if !line.dirty_line {
            continue;
        }

        let styled_line = ui_model::StyledLine::from(line, hl, ctx.font_features());
        let items = ctx.itemize(&styled_line);
        line.merge(&styled_line, &items);

        for (col, cell) in line.line.iter_mut().enumerate() {
            if cell.dirty {
                if let Some(item) = line.item_line[col].as_mut() {
                    let mut glyphs = pango::GlyphString::new();
                    {
                        let analysis = item.analysis();
                        let offset = item.item.offset() as usize;
                        let length = item.item.length() as usize;
                        if let Some(line_str) = styled_line.line_str.get(offset..offset + length) {
                            pango::shape(&line_str, analysis, &mut glyphs);
                        } else {
                            warn!("Wrong itemize split");
                        }
                    }

                    item.set_glyphs(ctx, glyphs);
                }
            }

            cell.dirty = false;
        }

        line.dirty_line = false;
    }
}
