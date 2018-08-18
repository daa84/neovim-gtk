use super::item::Item;
use super::UiModel;
use render::CellMetrics;

#[derive(Clone, Debug)]
pub struct ModelRectVec {
    pub list: Vec<ModelRect>,
}

impl ModelRectVec {
    pub fn empty() -> ModelRectVec {
        ModelRectVec { list: vec![] }
    }

    pub fn new(first: ModelRect) -> ModelRectVec {
        ModelRectVec { list: vec![first] }
    }

    fn find_neighbor(&self, neighbor: &ModelRect) -> Option<usize> {
        for (i, rect) in self.list.iter().enumerate() {
            if (neighbor.top > 0 && rect.top == neighbor.top - 1 || rect.bot == neighbor.bot + 1)
                && neighbor.in_horizontal(rect)
            {
                return Some(i);
            } else if (neighbor.left > 0 && rect.left == neighbor.left - 1
                || rect.right == neighbor.right + 1)
                && neighbor.in_vertical(rect)
            {
                return Some(i);
            } else if rect.in_horizontal(neighbor) && rect.in_vertical(neighbor) {
                return Some(i);
            } else if rect.contains(neighbor) {
                return Some(i);
            }
        }

        None
    }

    pub fn join(&mut self, other: &ModelRect) {
        match self.find_neighbor(other) {
            Some(i) => self.list[i].join(other),
            None => self.list.push(other.clone()),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct ModelRect {
    pub top: usize,
    pub bot: usize,
    pub left: usize,
    pub right: usize,
}

impl ModelRect {
    pub fn new(top: usize, bot: usize, left: usize, right: usize) -> ModelRect {
        debug_assert!(top <= bot, "{} <= {}", top, bot);
        debug_assert!(left <= right, "{} <= {}", left, right);

        ModelRect {
            top: top,
            bot: bot,
            left: left,
            right: right,
        }
    }

    pub fn point(x: usize, y: usize) -> ModelRect {
        ModelRect {
            top: y,
            bot: y,
            left: x,
            right: x,
        }
    }

    #[inline]
    fn in_horizontal(&self, other: &ModelRect) -> bool {
        other.left >= self.left && other.left <= self.right
            || other.right >= self.left && other.right >= self.right
    }

    #[inline]
    fn in_vertical(&self, other: &ModelRect) -> bool {
        other.top >= self.top && other.top <= self.bot
            || other.bot >= self.top && other.bot <= self.bot
    }

    fn contains(&self, other: &ModelRect) -> bool {
        self.top <= other.top
            && self.bot >= other.bot
            && self.left <= other.left
            && self.right >= other.right
    }

    /// Extend rect to left and right to make changed Item rerendered
    pub fn extend_by_items(&mut self, model: &UiModel) {
        let mut left = self.left;
        let mut right = self.right;

        for i in self.top..self.bot + 1 {
            let line = &model.model[i];
            let item_idx = line.cell_to_item(self.left);
            if item_idx >= 0 {
                let item_idx = item_idx as usize;
                if item_idx < left {
                    left = item_idx;
                }
            }

            let len_since_right = line.item_len_from_idx(self.right) - 1;
            if right < self.right + len_since_right {
                right = self.right + len_since_right;
            }

            // extend also double_width chars
            let cell = &line.line[self.left];
            if self.left > 0 && cell.double_width {
                let dw_char_idx = self.left - 1;
                if dw_char_idx < left {
                    left = dw_char_idx;
                }
            }

            let dw_char_idx = self.right + 1;
            if let Some(cell) = line.line.get(dw_char_idx) {
                if cell.double_width {
                    if right < dw_char_idx {
                        right = dw_char_idx;
                    }
                }
            }
        }

        self.left = left;
        self.right = right;
    }

    pub fn to_area_extend_ink(
        &self,
        model: &UiModel,
        cell_metrics: &CellMetrics,
    ) -> (i32, i32, i32, i32) {
        let (x, x2) = self.extend_left_right_area(model, cell_metrics);
        let (y, y2) = self.extend_top_bottom_area(model, cell_metrics);

        (x, y, x2 - x, y2 - y)
    }

    fn extend_left_right_area(&self, model: &UiModel, cell_metrics: &CellMetrics) -> (i32, i32) {
        let x = self.left as i32 * cell_metrics.char_width as i32;
        let x2 = (self.right + 1) as i32 * cell_metrics.char_width as i32;
        let mut min_x_offset = 0.0;
        let mut max_x_offset = 0.0;

        for row in self.top..self.bot + 1 {
            // left
            let line = &model.model[row];
            if let Some(&Item {
                ink_overflow: Some(ref overflow),
                ..
            }) = line.item_line[self.left].as_ref()
            {
                if min_x_offset < overflow.left {
                    min_x_offset = overflow.left;
                }
            }

            // right
            let line = &model.model[row];
            // check if this item ends here
            if self.right < model.columns - 1
                && line.cell_to_item(self.right) != line.cell_to_item(self.right + 1)
            {
                if let Some(&Item {
                    ink_overflow: Some(ref overflow),
                    ..
                }) = line.get_item(self.left)
                {
                    if max_x_offset < overflow.right {
                        max_x_offset = overflow.right;
                    }
                }
            }
        }

        (
            x - min_x_offset.ceil() as i32,
            x2 + max_x_offset.ceil() as i32,
        )
    }

    fn extend_top_bottom_area(&self, model: &UiModel, cell_metrics: &CellMetrics) -> (i32, i32) {
        let y = self.top as i32 * cell_metrics.line_height as i32;
        let y2 = (self.bot + 1) as i32 * cell_metrics.line_height as i32;
        let mut min_y_offset = 0.0;
        let mut max_y_offset = 0.0;

        for col in self.left..self.right + 1 {
            // top
            let line = &model.model[self.top];
            if let Some(&Item {
                ink_overflow: Some(ref overflow),
                ..
            }) = line.get_item(col)
            {
                if min_y_offset < overflow.top {
                    min_y_offset = overflow.top;
                }
            }

            // bottom
            let line = &model.model[self.bot];
            if let Some(&Item {
                ink_overflow: Some(ref overflow),
                ..
            }) = line.get_item(col)
            {
                if max_y_offset < overflow.top {
                    max_y_offset = overflow.top;
                }
            }
        }

        (
            y - min_y_offset.ceil() as i32,
            y2 + max_y_offset.ceil() as i32,
        )
    }

    pub fn join(&mut self, rect: &ModelRect) {
        self.top = if self.top < rect.top {
            self.top
        } else {
            rect.top
        };
        self.left = if self.left < rect.left {
            self.left
        } else {
            rect.left
        };

        self.bot = if self.bot > rect.bot {
            self.bot
        } else {
            rect.bot
        };
        self.right = if self.right > rect.right {
            self.right
        } else {
            rect.right
        };

        debug_assert!(self.top <= self.bot);
        debug_assert!(self.left <= self.right);
    }

    pub fn to_area(&self, cell_metrics: &CellMetrics) -> (i32, i32, i32, i32) {
        let &CellMetrics {
            char_width,
            line_height,
            ..
        } = cell_metrics;

        (
            self.left as i32 * char_width as i32,
            self.top as i32 * line_height as i32,
            (self.right - self.left + 1) as i32 * char_width as i32,
            (self.bot - self.top + 1) as i32 * line_height as i32,
        )
    }

    pub fn from_area(cell_metrics: &CellMetrics, x1: f64, y1: f64, x2: f64, y2: f64) -> ModelRect {
        let &CellMetrics {
            char_width,
            line_height,
            ..
        } = cell_metrics;

        let x2 = if x2 > 0.0 { x2 - 1.0 } else { x2 };
        let y2 = if y2 > 0.0 { y2 - 1.0 } else { y2 };
        let left = (x1 / char_width) as usize;
        let right = (x2 / char_width) as usize;
        let top = (y1 / line_height) as usize;
        let bot = (y2 / line_height) as usize;

        ModelRect::new(top, bot, left, right)
    }
}

impl AsRef<ModelRect> for ModelRect {
    fn as_ref(&self) -> &ModelRect {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repaint_rect() {
        let rect = ModelRect::point(1, 1);
        let (x, y, width, height) = rect.to_area(&CellMetrics::new_hw(10.0, 5.0));

        assert_eq!(5, x);
        assert_eq!(10, y);
        assert_eq!(5, width);
        assert_eq!(10, height);
    }

    #[test]
    fn test_from_area() {
        let rect = ModelRect::from_area(&CellMetrics::new_hw(10.0, 5.0), 3.0, 3.0, 9.0, 17.0);

        assert_eq!(0, rect.top);
        assert_eq!(0, rect.left);
        assert_eq!(1, rect.bot);
        assert_eq!(1, rect.right);

        let rect = ModelRect::from_area(&CellMetrics::new_hw(10.0, 5.0), 0.0, 0.0, 10.0, 20.0);

        assert_eq!(0, rect.top);
        assert_eq!(0, rect.left);
        assert_eq!(1, rect.bot);
        assert_eq!(1, rect.right);

        let rect = ModelRect::from_area(&CellMetrics::new_hw(10.0, 5.0), 0.0, 0.0, 11.0, 21.0);

        assert_eq!(0, rect.top);
        assert_eq!(0, rect.left);
        assert_eq!(2, rect.bot);
        assert_eq!(2, rect.right);
    }

}
