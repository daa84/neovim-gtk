use std::slice::Iter;
use cairo;

use super::context::CellMetrics;
use ui_model;

pub struct ModelClipIterator<'a> {
    model_idx: usize,
    model_iter: Iter<'a, ui_model::Line>,
}

pub trait ModelClipIteratorFactory {
    fn get_clip_iterator(
        &self,
        ctx: &cairo::Context,
        cell_metrics: &CellMetrics,
    ) -> ModelClipIterator;
}

impl<'a> Iterator for ModelClipIterator<'a> {
    type Item = (usize, &'a ui_model::Line);

    fn next(&mut self) -> Option<(usize, &'a ui_model::Line)> {
        let next = if let Some(line) = self.model_iter.next() {
            Some((self.model_idx, line))
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
    fn get_clip_iterator(
        &self,
        ctx: &cairo::Context,
        cell_metrics: &CellMetrics,
    ) -> ModelClipIterator {
        let model = self.model();

        let (x1, y1, x2, y2) = ctx.clip_extents();
        let model_clip = ui_model::ModelRect::from_area(cell_metrics, x1, y1, x2, y2);
        let model_clip_top = if model_clip.top <= 0 {
            0
        } else {
            model_clip.top - 1
        };
        let model_clip_bot = if model_clip.bot >= model.len() - 1 {
            model.len() - 1
        } else {
            model_clip.bot + 1
        };

        ModelClipIterator {
            model_idx: model_clip_top,
            model_iter: model[model_clip_top..model_clip_bot + 1].iter(),
        }
    }
}
