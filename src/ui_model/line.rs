use std::ops::{Index, IndexMut};

use pango;
use sys::pango as sys_pango;

use super::cell::Cell;
use super::item::Item;
use color;
use render;

pub struct Line {
    pub line: Box<[Cell]>,

    // format of item line is
    // [Item1, Item2, None, None, Item3]
    // Item2 take 3 cells and renders as one
    pub item_line: Box<[Option<Item>]>,
    cell_to_item: Box<[i32]>,

    pub dirty_line: bool,
}

impl Line {
    pub fn new(columns: usize) -> Self {
        Line {
            line: vec![Cell::new_empty(); columns].into_boxed_slice(),
            item_line: vec![None; columns].into_boxed_slice(),
            cell_to_item: vec![-1; columns].into_boxed_slice(),
            dirty_line: true,
        }
    }

    pub fn swap_with(&mut self, target: &mut Self, left: usize, right: usize) {
        // swap is faster then clone
        target.line[left..right + 1].swap_with_slice(&mut self.line[left..right + 1]);

        // this is because copy can change Item layout
        target.dirty_line = true;
        for cell in target.line[left..right + 1].iter_mut() {
            cell.dirty = true;
        }
    }

    pub fn clear(&mut self, left: usize, right: usize) {
        for cell in &mut self.line[left..right + 1] {
            cell.clear();
        }
        self.dirty_line = true;
    }

    pub fn clear_glyphs(&mut self) {
        for i in 0..self.item_line.len() {
            self.item_line[i] = None;
            self.cell_to_item[i] = -1;
        }
        self.dirty_line = true;
    }

    fn set_cell_to_empty(&mut self, cell_idx: usize) -> bool {
        if self.is_binded_to_item(cell_idx) {
            self.item_line[cell_idx] = None;
            self.cell_to_item[cell_idx] = -1;
            self.line[cell_idx].dirty = true;
            true
        } else {
            false
        }
    }

    fn set_cell_to_item(&mut self, new_item: &PangoItemPosition) -> bool {
        let start_item_idx = self.cell_to_item(new_item.start_cell);
        let start_item_cells_count = if start_item_idx >= 0 {
            self.item_line[start_item_idx as usize]
                .as_ref()
                .map_or(-1, |item| item.cells_count as i32)
        } else {
            -1
        };

        let end_item_idx = self.cell_to_item(new_item.end_cell);

        // start_item == idx of item start cell
        // in case different item length was in previous iteration
        // mark all item as dirty
        if start_item_idx != new_item.start_cell as i32
            || new_item.cells_count() != start_item_cells_count
            || start_item_idx == -1
            || end_item_idx == -1
        {
            self.initialize_cell_item(new_item.start_cell, new_item.end_cell, new_item.item);
            true
        } else {
            // update only if cell marked as dirty
            if self.line[new_item.start_cell..new_item.end_cell + 1]
                .iter()
                .any(|c| c.dirty)
            {
                self.item_line[new_item.start_cell]
                    .as_mut()
                    .unwrap()
                    .update(new_item.item.clone());
                self.line[new_item.start_cell].dirty = true;
                true
            } else {
                false
            }
        }
    }

    pub fn merge(&mut self, old_items: &StyledLine, pango_items: &[sys_pango::Item]) {
        let mut pango_item_iter = pango_items
            .iter()
            .map(|item| PangoItemPosition::new(old_items, item));

        let mut next_item = pango_item_iter.next();
        let mut move_to_next_item = false;

        let mut cell_idx = 0;
        while cell_idx < self.line.len() {
            let dirty = match next_item {
                None => self.set_cell_to_empty(cell_idx),
                Some(ref new_item) => {
                    if cell_idx < new_item.start_cell {
                        self.set_cell_to_empty(cell_idx)
                    } else if cell_idx == new_item.start_cell {
                        move_to_next_item = true;
                        self.set_cell_to_item(new_item)
                    } else {
                        false
                    }
                }
            };

            self.dirty_line = self.dirty_line || dirty;
            if move_to_next_item {
                let new_item = next_item.unwrap();
                cell_idx += new_item.end_cell - new_item.start_cell + 1;
                next_item = pango_item_iter.next();
                move_to_next_item = false;
            } else {
                cell_idx += 1;
            }
        }
    }

    fn initialize_cell_item(
        &mut self,
        start_cell: usize,
        end_cell: usize,
        new_item: &sys_pango::Item,
    ) {
        for i in start_cell..end_cell + 1 {
            self.line[i].dirty = true;
            self.cell_to_item[i] = start_cell as i32;
        }
        for i in start_cell + 1..end_cell + 1 {
            self.item_line[i] = None;
        }
        self.item_line[start_cell] = Some(Item::new(new_item.clone(), end_cell - start_cell + 1));
    }

    pub fn get_item(&self, cell_idx: usize) -> Option<&Item> {
        let item_idx = self.cell_to_item(cell_idx);
        if item_idx >= 0 {
            self.item_line[item_idx as usize].as_ref()
        } else {
            None
        }
    }

    #[inline]
    pub fn cell_to_item(&self, cell_idx: usize) -> i32 {
        self.cell_to_item[cell_idx]
    }

    pub fn item_len_from_idx(&self, start_idx: usize) -> usize {
        debug_assert!(
            start_idx < self.line.len(),
            "idx={}, len={}",
            start_idx,
            self.line.len()
        );

        let item_idx = self.cell_to_item(start_idx);

        if item_idx >= 0 {
            let item_idx = item_idx as usize;
            let cells_count = self.item_line[item_idx].as_ref().unwrap().cells_count;
            let offset = start_idx - item_idx;

            cells_count - offset
        } else {
            1
        }
    }

    #[inline]
    pub fn is_binded_to_item(&self, cell_idx: usize) -> bool {
        self.cell_to_item[cell_idx] >= 0
    }
}

impl Index<usize> for Line {
    type Output = Cell;

    fn index(&self, index: usize) -> &Cell {
        &self.line[index]
    }
}

impl IndexMut<usize> for Line {
    fn index_mut(&mut self, index: usize) -> &mut Cell {
        &mut self.line[index]
    }
}

struct PangoItemPosition<'a> {
    item: &'a sys_pango::Item,
    start_cell: usize,
    end_cell: usize,
}

impl<'a> PangoItemPosition<'a> {
    pub fn new(styled_line: &StyledLine, item: &'a sys_pango::Item) -> Self {
        let (offset, length, _) = item.offset();
        let start_cell = styled_line.cell_to_byte[offset];
        let end_cell = styled_line.cell_to_byte[offset + length - 1];

        PangoItemPosition {
            item,
            start_cell,
            end_cell,
        }
    }

    #[inline]
    fn cells_count(&self) -> i32 {
        (self.end_cell - self.start_cell) as i32 + 1
    }
}

pub struct StyledLine {
    pub line_str: String,
    cell_to_byte: Box<[usize]>,
    pub attr_list: pango::AttrList,
}

impl StyledLine {
    pub fn from(
        line: &Line,
        color_model: &color::ColorModel,
        font_features: &render::FontFeatures,
    ) -> Self {
        let average_capacity = line.line.len() * 4 * 2; // code bytes * grapheme cluster

        let mut line_str = String::with_capacity(average_capacity);
        let mut cell_to_byte = Vec::with_capacity(average_capacity);
        let attr_list = pango::AttrList::new();
        let mut byte_offset = 0;
        let mut style_attr = StyleAttr::new();

        for (cell_idx, cell) in line.line.iter().enumerate() {
            if cell.attrs.double_width {
                continue;
            }

            if !cell.ch.is_empty() {
                line_str.push_str(&cell.ch);
            } else {
                line_str.push(' ');
            }
            let len = line_str.len() - byte_offset;

            for _ in 0..len {
                cell_to_byte.push(cell_idx);
            }

            let next = style_attr.next(byte_offset, byte_offset + len, cell, color_model);
            if let Some(next) = next {
                style_attr.insert(&attr_list);
                style_attr = next;
            }

            byte_offset += len;
        }

        style_attr.insert(&attr_list);
        font_features.insert_attr(&attr_list);

        StyledLine {
            line_str,
            cell_to_byte: cell_to_byte.into_boxed_slice(),
            attr_list,
        }
    }
}

struct StyleAttr<'c> {
    italic: bool,
    bold: bool,
    foreground: Option<&'c color::Color>,
    background: Option<&'c color::Color>,
    empty: bool,

    start_idx: usize,
    end_idx: usize,
}

impl<'c> StyleAttr<'c> {
    fn new() -> Self {
        StyleAttr {
            italic: false,
            bold: false,
            foreground: None,
            background: None,
            empty: true,

            start_idx: 0,
            end_idx: 0,
        }
    }

    fn from(
        start_idx: usize,
        end_idx: usize,
        cell: &'c Cell,
        color_model: &'c color::ColorModel,
    ) -> Self {
        StyleAttr {
            italic: cell.attrs.italic,
            bold: cell.attrs.bold,
            foreground: color_model.cell_fg(cell),
            background: color_model.cell_bg(cell),
            empty: false,

            start_idx,
            end_idx,
        }
    }

    fn next(
        &mut self,
        start_idx: usize,
        end_idx: usize,
        cell: &'c Cell,
        color_model: &'c color::ColorModel,
    ) -> Option<StyleAttr<'c>> {
        let style_attr = Self::from(start_idx, end_idx, cell, color_model);

        if self != &style_attr {
            Some(style_attr)
        } else {
            self.end_idx = end_idx;
            None
        }
    }

    fn insert(&self, attr_list: &pango::AttrList) {
        if self.empty {
            return;
        }

        if self.italic {
            self.insert_attr(
                attr_list,
                pango::Attribute::new_style(pango::Style::Italic).unwrap(),
            );
        }

        if self.bold {
            self.insert_attr(
                attr_list,
                pango::Attribute::new_weight(pango::Weight::Bold).unwrap(),
            );
        }

        if let Some(fg) = self.foreground {
            let (r, g, b) = fg.to_u16();
            self.insert_attr(
                attr_list,
                pango::Attribute::new_foreground(r, g, b).unwrap(),
            );
        }

        if let Some(bg) = self.background {
            let (r, g, b) = bg.to_u16();
            self.insert_attr(
                attr_list,
                pango::Attribute::new_background(r, g, b).unwrap(),
            );
        }
    }

    #[inline]
    fn insert_attr(&self, attr_list: &pango::AttrList, mut attr: pango::Attribute) {
        attr.set_start_index(self.start_idx as u32);
        attr.set_end_index(self.end_idx as u32);
        attr_list.insert(attr);
    }
}

impl<'c> PartialEq for StyleAttr<'c> {
    fn eq(&self, other: &Self) -> bool {
        self.italic == other.italic
            && self.bold == other.bold
            && self.foreground == other.foreground
            && self.empty == other.empty
            && self.background == other.background
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_styled_line() {
        let mut line = Line::new(3);
        line[0].ch = "a".to_owned();
        line[1].ch = "b".to_owned();
        line[2].ch = "c".to_owned();

        let styled_line = StyledLine::from(
            &line,
            &color::ColorModel::new(),
            &render::FontFeatures::new(),
        );
        assert_eq!("abc", styled_line.line_str);
        assert_eq!(3, styled_line.cell_to_byte.len());
        assert_eq!(0, styled_line.cell_to_byte[0]);
        assert_eq!(1, styled_line.cell_to_byte[1]);
        assert_eq!(2, styled_line.cell_to_byte[2]);
    }
}
