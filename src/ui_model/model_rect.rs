use std::slice::Iter;

use super::{Line, UiModel, Cell};


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

    /// Extend rect to left and right to make change Item rerendered
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
        }
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
    iter: Iter<'a, Line>,
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
    line: &'a Line,
}

impl<'a> ClipLine<'a> {
    pub fn new(model: &'a Line, rect: &'a ModelRect) -> ClipLine<'a> {
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
        self.line.line.get(idx)
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
    pub fn new(model: &'a Line, rect: &'a ModelRect) -> ClipColIterator<'a> {
        ClipColIterator {
            rect: rect,
            pos: 0,
            iter: model.line[rect.left..rect.right + 1].iter(),
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
