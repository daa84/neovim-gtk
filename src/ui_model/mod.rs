use highlight::Highlight;
use std::rc::Rc;

mod cell;
mod item;
mod line;
mod model_layout;
mod model_rect;

pub use self::cell::Cell;
pub use self::item::Item;
pub use self::line::{Line, StyledLine};
pub use self::model_layout::ModelLayout;
pub use self::model_rect::{ModelRect, ModelRectVec};

pub struct UiModel {
    pub columns: usize,
    pub rows: usize,
    cur_row: usize,
    cur_col: usize,
    model: Box<[Line]>,
}

impl UiModel {
    pub fn new(rows: u64, columns: u64) -> UiModel {
        let mut model = Vec::with_capacity(rows as usize);
        for _ in 0..rows as usize {
            model.push(Line::new(columns as usize));
        }

        UiModel {
            columns: columns as usize,
            rows: rows as usize,
            cur_row: 0,
            cur_col: 0,
            model: model.into_boxed_slice(),
        }
    }

    pub fn empty() -> UiModel {
        UiModel {
            columns: 0,
            rows: 0,
            cur_row: 0,
            cur_col: 0,
            model: Box::new([]),
        }
    }

    #[inline]
    pub fn model(&self) -> &[Line] {
        &self.model
    }

    #[inline]
    pub fn model_mut(&mut self) -> &mut [Line] {
        &mut self.model
    }

    pub fn cur_point(&self) -> ModelRect {
        ModelRect::point(self.cur_col, self.cur_row)
    }

    pub fn set_cursor(&mut self, row: usize, col: usize) -> ModelRectVec {
        // it is possible in some cases that cursor moved out of visible rect
        // see https://github.com/daa84/neovim-gtk/issues/20
        if row >= self.model.len() || col >= self.model[row].line.len() {
            return ModelRectVec::empty();
        }

        let mut changed_region = ModelRectVec::new(self.cur_point());

        self.cur_row = row;
        self.cur_col = col;

        changed_region.join(&self.cur_point());

        changed_region
    }

    pub fn get_cursor(&self) -> (usize, usize) {
        (self.cur_row, self.cur_col)
    }

    pub fn put_one(&mut self, row: usize, col: usize, ch: &str, double_width: bool, hl: Option<&Highlight>) {
        let hl = hl.map(|hl| Rc::new(hl.clone())).unwrap_or_else(|| Rc::new(Highlight::new()));
        self.put(row, col, ch, double_width, 1, hl);
    }

    pub fn put(
        &mut self,
        row: usize,
        col: usize,
        ch: &str,
        double_width: bool,
        repeat: usize,
        hl: Rc<Highlight>,
    ) {
        let line = &mut self.model[row];
        line.dirty_line = true;

        for offset in 0..repeat {
            let cell = &mut line[col + offset];
            cell.ch.clear();
            cell.ch.push_str(ch);
            cell.hl = hl.clone();
            cell.double_width = double_width;
        }
    }

    //    pub fn put(&mut self, ch: &str, double_width: bool, attrs: Option<&Attrs>) -> ModelRect {
    //        let mut changed_region = self.cur_point();
    //        let line = &mut self.model[self.cur_row];
    //        line.dirty_line = true;
    //
    //        let cell = &mut line[self.cur_col];
    //
    //        cell.ch.clear();
    //        cell.ch.push_str(ch);
    //
    //        cell.attrs = attrs.map(Attrs::clone).unwrap_or_else(Attrs::new);
    //        cell.attrs.double_width = double_width;
    //        cell.dirty = true;
    //        self.cur_col += 1;
    //        if self.cur_col >= self.columns {
    //            self.cur_col -= 1;
    //        }
    //
    //        changed_region.join(&ModelRect::point(self.cur_col, self.cur_row));
    //
    //        changed_region
    //    }

    /// Copy rows from 0 to to_row, col from 0 self.columns
    ///
    /// Don't do any validation!
    pub fn swap_rows(&mut self, target: &mut UiModel, to_row: usize) {
        for (row_idx, line) in self.model[0..to_row + 1].iter_mut().enumerate() {
            let mut target_row = &mut target.model[row_idx];
            line.swap_with(target_row, 0, self.columns - 1);
        }
    }

    #[inline]
    fn swap_row(&mut self, target_row: i64, offset: i64, left_col: usize, right_col: usize) {
        debug_assert_ne!(0, offset);

        let from_row = (target_row + offset) as usize;

        let (left, right) = if offset > 0 {
            self.model.split_at_mut(from_row)
        } else {
            self.model.split_at_mut(target_row as usize)
        };

        let (source_row, target_row) = if offset > 0 {
            (&mut right[0], &mut left[target_row as usize])
        } else {
            (&mut left[from_row], &mut right[0])
        };

        source_row.swap_with(target_row, left_col, right_col);
    }

    pub fn scroll(&mut self, top: i64, bot: i64, left: usize, right: usize, count: i64, default_hl: &Rc<Highlight>) -> ModelRect {
        if count > 0 {
            for row in top..(bot - count) {
                self.swap_row(row, count, left, right);
            }
        } else {
            for row in ((top - count)..(bot)).rev() {
                self.swap_row(row, count, left, right);
            }
        }

        if count > 0 {
            self.clear_region((bot - count + 1) as usize, bot as usize, left, right, default_hl);
        } else {
            self.clear_region(top as usize, (top - count - 1) as usize, left, right, default_hl);
        }

        ModelRect::new(top as usize, bot as usize, left, right)
    }

    pub fn clear(&mut self, default_hl: &Rc<Highlight>) {
        let (rows, columns) = (self.rows, self.columns);
        self.clear_region(0, rows - 1, 0, columns - 1, default_hl);
    }

    fn clear_region(&mut self, top: usize, bot: usize, left: usize, right: usize, default_hl: &Rc<Highlight>) {
        for row in &mut self.model[top..bot] {
            row.clear(left, right, default_hl);
        }
    }

    pub fn clear_glyphs(&mut self) {
        for row in &mut self.model.iter_mut() {
            row.clear_glyphs();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_join_inside() {
        let mut list = ModelRectVec::new(ModelRect::new(0, 23, 0, 69));

        let inside = ModelRect::new(23, 23, 68, 69);

        list.join(&inside);
        assert_eq!(1, list.list.len());
    }

    #[test]
    fn test_vec_join_top() {
        let mut list = ModelRectVec::new(ModelRect::point(0, 0));

        let neighbor = ModelRect::point(1, 0);

        list.join(&neighbor);
        assert_eq!(1, list.list.len());
    }

    #[test]
    fn test_model_vec_join_right() {
        let mut list = ModelRectVec::new(ModelRect::new(23, 23, 69, 69));

        let neighbor = ModelRect::new(23, 23, 69, 70);

        list.join(&neighbor);
        assert_eq!(1, list.list.len());
    }

    #[test]
    fn test_model_vec_join_right2() {
        let mut list = ModelRectVec::new(ModelRect::new(0, 1, 0, 9));

        let neighbor = ModelRect::new(1, 1, 9, 10);

        list.join(&neighbor);
        assert_eq!(1, list.list.len());
    }

    #[test]
    fn test_model_vec_join() {
        let mut list = ModelRectVec::new(ModelRect::point(5, 5));

        let neighbor = ModelRect::point(6, 5);

        list.join(&neighbor);
        assert_eq!(1, list.list.len());
    }

    #[test]
    fn test_model_vec_no_join() {
        let mut list = ModelRectVec::new(ModelRect::point(5, 5));

        let not_neighbor = ModelRect::point(6, 6);

        list.join(&not_neighbor);
        assert_eq!(2, list.list.len());
    }

    #[test]
    fn test_cursor_area() {
        let mut model = UiModel::new(10, 20);

        model.set_cursor(1, 1);

        let rect = model.set_cursor(5, 5);

        assert_eq!(2, rect.list.len());

        assert_eq!(1, rect.list[0].top);
        assert_eq!(1, rect.list[0].left);
        assert_eq!(1, rect.list[0].bot);
        assert_eq!(1, rect.list[0].right);

        assert_eq!(5, rect.list[1].top);
        assert_eq!(5, rect.list[1].left);
        assert_eq!(5, rect.list[1].bot);
        assert_eq!(5, rect.list[1].right);
    }

    #[test]
    fn test_eol_clear_area() {
        let mut model = UiModel::new(10, 20);

        model.set_cursor(1, 2);

        let rect = model.eol_clear();

        assert_eq!(1, rect.top);
        assert_eq!(2, rect.left);
        assert_eq!(1, rect.bot);
        assert_eq!(19, rect.right);
    }

    #[test]
    fn test_put_area() {
        let mut model = UiModel::new(10, 20);

        model.set_cursor(1, 1);

        let rect = model.put(" ", false, None);

        assert_eq!(1, rect.top);
        assert_eq!(1, rect.left);
        assert_eq!(1, rect.bot);
        assert_eq!(2, rect.right);
    }

    #[test]
    fn test_scroll_area() {
        let mut model = UiModel::new(10, 20);

        model.set_scroll_region(1, 5, 1, 5);

        let rect = model.scroll(3);

        assert_eq!(1, rect.top);
        assert_eq!(1, rect.left);
        assert_eq!(5, rect.bot);
        assert_eq!(5, rect.right);
    }
}
