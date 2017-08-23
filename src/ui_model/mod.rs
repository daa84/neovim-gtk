mod cell;
mod line;

pub use self::cell::{Cell, Attrs};
use self::line::Line;

use std::slice::Iter;

pub struct UiModel {
    pub columns: usize,
    pub rows: usize,
    cur_row: usize,
    cur_col: usize,
    model: Box<[Line]>,
    top: usize,
    bot: usize,
    left: usize,
    right: usize,
}

impl UiModel {
    pub fn new(rows: u64, columns: u64) -> UiModel {
        let mut model = Vec::with_capacity(rows as usize);
        for i in 0..rows as usize {
            model.push(Line::new(columns as usize));
        }

        UiModel {
            columns: columns as usize,
            rows: rows as usize,
            cur_row: 0,
            cur_col: 0,
            model: model.into_boxed_slice(),
            top: 0,
            bot: (rows - 1) as usize,
            left: 0,
            right: (columns - 1) as usize,
        }
    }

    pub fn model(&self) -> &[Line] {
        &self.model
    }

    pub fn limit_to_model(&self, clip: &mut ModelRect) {
        clip.left = if clip.left >= self.columns {
            self.columns - 1
        } else {
            clip.left
        };
        clip.right = if clip.right >= self.columns {
            self.columns - 1
        } else {
            clip.right
        };
        clip.top = if clip.top >= self.rows {
            self.rows - 1
        } else {
            clip.top
        };
        clip.bot = if clip.bot >= self.rows {
            self.rows - 1
        } else {
            clip.bot
        };
    }

    pub fn clip_model<'a>(&'a self, clip: &'a ModelRect) -> ClipRowIterator<'a> {
        ClipRowIterator::new(self, clip)
    }

    pub fn cur_point(&self) -> ModelRect {
        ModelRect::point(self.cur_col, self.cur_row)
    }

    pub fn set_cursor(&mut self, row: usize, col: usize) -> ModelRectVec {
        let mut changed_region = ModelRectVec::new(self.cur_point());

        self.cur_row = row;
        self.cur_col = col;

        changed_region.join(&self.cur_point());

        changed_region

    }

    pub fn get_cursor(&self) -> (usize, usize) {
        (self.cur_row, self.cur_col)
    }

    pub fn put(&mut self, text: &str, attrs: Option<&Attrs>) -> ModelRect {
        let mut changed_region = self.cur_point();
        let mut cell = &mut self.model[self.cur_row][self.cur_col];

        cell.ch = text.chars().last().unwrap_or(' ');
        cell.attrs = attrs.map(Attrs::clone).unwrap_or_else(Attrs::new);
        cell.attrs.double_width = text.is_empty();
        self.cur_col += 1;
        if self.cur_col >= self.columns {
            self.cur_col -= 1;
        }

        changed_region.join(&ModelRect::point(self.cur_col, self.cur_row));

        changed_region
    }

    pub fn set_scroll_region(&mut self, top: u64, bot: u64, left: u64, right: u64) {
        self.top = top as usize;
        self.bot = bot as usize;
        self.left = left as usize;
        self.right = right as usize;
    }

    #[inline]
    fn copy_row(&mut self, row: i64, offset: i64, left: usize, right: usize) {
        for col in left..right + 1 {
            let from_row = (row + offset) as usize;
            let from_cell = self.model[from_row][col].clone();
            self.model[row as usize][col] = from_cell;
        }
    }

    pub fn scroll(&mut self, count: i64) -> ModelRect {
        let (top, bot, left, right) = (self.top as i64, self.bot as i64, self.left, self.right);

        if count > 0 {
            for row in top..(bot - count + 1) {
                self.copy_row(row, count, left, right);
            }
        } else {
            for row in ((top - count)..(bot + 1)).rev() {
                self.copy_row(row, count, left, right);
            }
        }

        if count > 0 {
            self.clear_region((bot - count + 1) as usize, bot as usize, left, right);
        } else {
            self.clear_region(top as usize, (top - count - 1) as usize, left, right);
        }

        ModelRect::new(top as usize, bot as usize, left, right)
    }

    pub fn clear(&mut self) {
        let (rows, columns) = (self.rows, self.columns);
        self.clear_region(0, rows - 1, 0, columns - 1);
    }

    pub fn eol_clear(&mut self) -> ModelRect {
        let (cur_row, cur_col, columns) = (self.cur_row, self.cur_col, self.columns);
        self.clear_region(cur_row, cur_row, cur_col, columns - 1);

        ModelRect::new(cur_row, cur_row, cur_col, columns - 1)
    }

    fn clear_region(&mut self, top: usize, bot: usize, left: usize, right: usize) {
        for row in &mut self.model[top..bot + 1] {
            for cell in &mut row[left..right + 1] {
                cell.clear();
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModelRectVec {
    pub list: Vec<ModelRect>,
}

impl ModelRectVec {
    pub fn new(first: ModelRect) -> ModelRectVec {
        ModelRectVec { list: vec![first] }
    }

    fn find_neighbor(&self, neighbor: &ModelRect) -> Option<usize> {
        for (i, rect) in self.list.iter().enumerate() {
            if (neighbor.top > 0 && rect.top == neighbor.top - 1 ||
                rect.bot == neighbor.bot + 1) && neighbor.in_horizontal(rect) {
                return Some(i);
            } else if (neighbor.left > 0 && rect.left == neighbor.left - 1 ||
                       rect.right == neighbor.right + 1) &&
                      neighbor.in_vertical(rect) {
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
        debug_assert!(top <= bot);
        debug_assert!(left <= right);

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
        other.left >= self.left && other.left <= self.right ||
        other.right >= self.left && other.right >= self.right
    }

    #[inline]
    fn in_vertical(&self, other: &ModelRect) -> bool {
        other.top >= self.top && other.top <= self.bot ||
        other.bot >= self.top && other.bot <= self.bot
    }

    fn contains(&self, other: &ModelRect) -> bool {
        self.top <= other.top && self.bot >= other.bot && self.left <= other.left &&
        self.right >= other.right
    }

    pub fn extend(&mut self, top: usize, bot: usize, left: usize, right: usize) {
        if self.top > 0 {
            self.top -= top;
        }
        if self.left > 0 {
            self.left -= left;
        }
        self.bot += bot;
        self.right += right;
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

    pub fn to_area(&self, line_height: f64, char_width: f64) -> (i32, i32, i32, i32) {
        (self.left as i32 * char_width as i32,
         self.top as i32 * line_height as i32,
         (self.right - self.left + 1) as i32 * char_width as i32,
         (self.bot - self.top + 1) as i32 * line_height as i32)
    }

    pub fn from_area(line_height: f64,
                     char_width: f64,
                     x1: f64,
                     y1: f64,
                     x2: f64,
                     y2: f64)
                     -> ModelRect {
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

pub struct ClipRowIterator<'a> {
    rect: &'a ModelRect,
    pos: usize,
    iter: Iter<'a, Vec<Cell>>,
}

impl<'a> ClipRowIterator<'a> {
    pub fn new(model: &'a UiModel, rect: &'a ModelRect) -> ClipRowIterator<'a> {
        ClipRowIterator {
            rect: rect,
            pos: 0,
            iter: model.model()[rect.top..rect.bot + 1].iter(),
        }
    }
}

impl<'a> Iterator for ClipRowIterator<'a> {
    type Item = (usize, ClipLine<'a>);

    fn next(&mut self) -> Option<(usize, ClipLine<'a>)> {
        self.pos += 1;
        self.iter
            .next()
            .map(|line| (self.rect.top + self.pos - 1, ClipLine::new(line, self.rect)))
    }
}

pub struct ClipLine<'a> {
    rect: &'a ModelRect,
    line: &'a [Cell],
}

impl<'a> ClipLine<'a> {
    pub fn new(model: &'a [Cell], rect: &'a ModelRect) -> ClipLine<'a> {
        ClipLine {
            line: model,
            rect: rect,
        }
    }

    #[inline]
    pub fn is_double_width(&self, col_idx: usize) -> bool {
        self.get(col_idx + 1)
            .map(|c| c.attrs.double_width)
            .unwrap_or(false)
    }

    pub fn get(&self, idx: usize) -> Option<&Cell> {
        self.line.get(idx)
    }

    pub fn iter(&self) -> ClipColIterator<'a> {
        ClipColIterator::new(self.line, self.rect)
    }
}

pub struct ClipColIterator<'a> {
    rect: &'a ModelRect,
    pos: usize,
    iter: Iter<'a, Cell>,
}

impl<'a> ClipColIterator<'a> {
    pub fn new(model: &'a [Cell], rect: &'a ModelRect) -> ClipColIterator<'a> {
        ClipColIterator {
            rect: rect,
            pos: 0,
            iter: model[rect.left..rect.right + 1].iter(),
        }
    }
}

impl<'a> Iterator for ClipColIterator<'a> {
    type Item = (usize, &'a Cell);

    fn next(&mut self) -> Option<(usize, &'a Cell)> {
        self.pos += 1;
        self.iter
            .next()
            .map(|line| (self.rect.left + self.pos - 1, line))
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
    fn test_iterator_border() {
        let model = UiModel::new(10, 20);
        let rect = ModelRect::new(0, 9, 0, 19);

        assert_eq!(10, model.clip_model(&rect).count());
        let (_, first_line) = model.clip_model(&rect).nth(0).unwrap();
        assert_eq!(20, first_line.iter().count());

        let (idx, _) = first_line.iter().nth(19).unwrap();
        assert_eq!(19, idx);
    }

    #[test]
    fn test_iterator() {
        let model = UiModel::new(10, 20);
        let rect = ModelRect::new(1, 2, 1, 2);

        assert_eq!(2, model.clip_model(&rect).count());
        let (idx, first_line) = model.clip_model(&rect).nth(0).unwrap();

        assert_eq!(1, idx);
        assert_eq!(2, first_line.iter().count());

        let (idx, _) = first_line.iter().nth(0).unwrap();
        assert_eq!(1, idx);
    }

    #[test]
    fn test_from_area() {
        let rect = ModelRect::from_area(10.0, 5.0, 3.0, 3.0, 9.0, 17.0);

        assert_eq!(0, rect.top);
        assert_eq!(0, rect.left);
        assert_eq!(1, rect.bot);
        assert_eq!(1, rect.right);


        let rect = ModelRect::from_area(10.0, 5.0, 0.0, 0.0, 10.0, 20.0);

        assert_eq!(0, rect.top);
        assert_eq!(0, rect.left);
        assert_eq!(1, rect.bot);
        assert_eq!(1, rect.right);


        let rect = ModelRect::from_area(10.0, 5.0, 0.0, 0.0, 11.0, 21.0);

        assert_eq!(0, rect.top);
        assert_eq!(0, rect.left);
        assert_eq!(2, rect.bot);
        assert_eq!(2, rect.right);
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
    fn test_repaint_rect() {
        let rect = ModelRect::point(1, 1);
        let (x, y, width, height) = rect.to_area(10.0, 5.0);

        assert_eq!(5, x);
        assert_eq!(10, y);
        assert_eq!(5, width);
        assert_eq!(10, height);
    }

    #[test]
    fn test_put_area() {
        let mut model = UiModel::new(10, 20);

        model.set_cursor(1, 1);

        let rect = model.put(" ", None);

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
