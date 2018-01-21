use std::cmp::min;
use std::slice::Iter;

use cairo;

use super::context::CellMetrics;
use ui_model;

pub struct RowView<'a> {
    pub row: usize,
    pub line: &'a ui_model::Line,
    pub cell_metrics: &'a CellMetrics,
    pub line_y: f64,
    pub ctx: &'a cairo::Context,
}

impl<'a> RowView<'a> {
    pub fn new(
        row: usize,
        ctx: &'a cairo::Context,
        cell_metrics: &'a CellMetrics,
        line: &'a ui_model::Line,
    ) -> Self {
        RowView {
            line,
            line_y: row as f64 * cell_metrics.line_height,
            row,
            cell_metrics,
            ctx,
        }
    }
}

pub struct ModelClipIterator<'a> {
    model_idx: usize,
    model_iter: Iter<'a, ui_model::Line>,
    cell_metrics: &'a CellMetrics,
    ctx: &'a cairo::Context,
}

pub trait ModelClipIteratorFactory {
    fn get_clip_iterator<'a>(
        &'a self,
        ctx: &'a cairo::Context,
        cell_metrics: &'a CellMetrics,
    ) -> ModelClipIterator;
}

impl<'a> Iterator for ModelClipIterator<'a> {
    type Item = RowView<'a>;

    fn next(&mut self) -> Option<RowView<'a>> {
        let next = if let Some(line) = self.model_iter.next() {
            Some(RowView::new(
                self.model_idx,
                self.ctx,
                self.cell_metrics,
                line,
            ))
        } else {
            None
        };
        self.model_idx += 1;

        next
    }
}

/// Clip implemented as top - 1/bot + 1
/// this is because in some cases(like 'g' character) drawing character does not fit to calculated bounds
/// and if one line must be repainted - also previous and next line must be repainted to
impl ModelClipIteratorFactory for ui_model::UiModel {
    fn get_clip_iterator<'a>(
        &'a self,
        ctx: &'a cairo::Context,
        cell_metrics: &'a CellMetrics,
    ) -> ModelClipIterator<'a> {
        let model = self.model();

        let (x1, y1, x2, y2) = ctx.clip_extents();

        // in case ctx.translate is used y1 can be less then 0
        // in this case just use 0 as top value
        let model_clip = ui_model::ModelRect::from_area(cell_metrics, x1, y1.max(0.0), x2, y2);

        let model_clip_top = if model_clip.top == 0 {
            0
        } else {
            model_clip.top - 1
        };
        let model_clip_bot = min(model.len() - 1, model_clip.bot + 1);

        ModelClipIterator {
            model_idx: model_clip_top,
            model_iter: model[model_clip_top..model_clip_bot + 1].iter(),
            ctx,
            cell_metrics,
        }
    }
}
