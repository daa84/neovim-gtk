use std::ops::{Index, IndexMut};

use super::cell::Cell;
use sys::pango as sys_pango;
use pango;

pub struct Item {
    pub item: sys_pango::Item,
    pub glyphs: Option<pango::GlyphString>,
    pub ink_rect: Option<pango::Rectangle>,
    font: pango::Font,
}

impl Item {
    pub fn new(item: sys_pango::Item) -> Self {
        Item {
            font: item.analysis().font(),
            item,
            glyphs: None,
            ink_rect: None,
        }
    }

    pub fn update(&mut self, item: sys_pango::Item) {
        self.font = item.analysis().font();
        self.item = item;
        self.glyphs = None;
        self.ink_rect = None;
    }

    pub fn set_glyphs(&mut self, glyphs: pango::GlyphString) {
        let mut glyphs = glyphs;
        let (ink_rect, _) = glyphs.extents(&self.font);
        self.ink_rect = Some(ink_rect);
        self.glyphs = Some(glyphs);
    }

    pub fn font(&self) -> &pango::Font {
        &self.font
    }

    pub fn analysis(&self) -> sys_pango::Analysis {
        self.item.analysis()
    }
}

pub struct Line {
    pub line: Box<[Cell]>,

    // format of item line is
    // [Item1, Item2, None, None, Item3]
    // Item2 take 3 cells and renders as one
    pub item_line: Box<[Option<Item>]>,
    cell_to_item: Box<[i32]>,

    item_line_empty: bool,
    pub dirty_line: bool,
}

impl Line {
    pub fn new(columns: usize) -> Self {
        let mut line = Vec::with_capacity(columns);
        for _ in 0..columns {
            line.push(Cell::new(' '));
        }
        let mut item_line = Vec::with_capacity(columns);
        for _ in 0..columns {
            item_line.push(None);
        }

        Line {
            line: line.into_boxed_slice(),
            item_line: item_line.into_boxed_slice(),
            cell_to_item: vec![-1; columns].into_boxed_slice(),
            dirty_line: false,
            item_line_empty: true,
        }
    }

    pub fn clear(&mut self, left: usize, right: usize) {
        for cell in &mut self.line[left..right + 1] {
            cell.clear();
        }
    }

    pub fn merge(&mut self, old_items: &StyledLine, new_items: &[sys_pango::Item]) {
        for new_item in new_items {
            //FIXME: clear empty cells
            let (offset, length, _) = new_item.offset();
            let start_cell = old_items.cell_to_byte[offset];
            let end_cell = old_items.cell_to_byte[offset + length - 1];

            // first time initialization
            // as cell_to_item points to wrong values
            if !self.item_line_empty {
                let start_item = self.cell_to_item(start_cell);
                let end_item = self.cell_to_item(end_cell);

                // in case different item length was in previous iteration
                // mark all item as dirty
                if start_item != end_item {
                    self.initialize_cell_item(start_cell, end_cell, new_item);
                } else {
                    self.item_line[start_cell].as_mut().unwrap().update(
                        new_item.clone(),
                    );
                }
            } else {
                self.initialize_cell_item(start_cell, end_cell, new_item);
            }
        }

        self.item_line_empty = false;
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
        for i in start_cell + 1..end_cell {
            self.item_line[i] = None;
        }
        self.item_line[start_cell] = Some(Item::new(new_item.clone()));
    }

    pub fn mark_dirty_cell(&mut self, idx: usize) {
        self.line[idx].dirty = true;
    }

    pub fn get_item_mut(&mut self, cell_idx: usize) -> Option<&mut Item> {
        let item_idx = self.cell_to_item(cell_idx);
        if item_idx >= 0 {
            self.item_line[item_idx as usize].as_mut()
        } else {
            None
        }
    }

    fn cell_to_item(&self, cell_idx: usize) -> i32 {
        self.cell_to_item[cell_idx]
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

pub struct StyledLine {
    pub line_str: String,
    cell_to_byte: Box<[usize]>,
    pub attr_list: pango::AttrList,
}

impl StyledLine {
    pub fn from(line: &Line) -> Self {
        let mut line_str = String::new();
        let mut cell_to_byte = Vec::new();
        let attr_list = pango::AttrList::new();
        let mut byte_offset = 0;

        for (cell_idx, cell) in line.line.iter().enumerate() {
            if cell.attrs.double_width {
                continue;
            }

            line_str.push(cell.ch);
            let len = line_str.len() - byte_offset;

            for _ in 0..len {
                cell_to_byte.push(cell_idx);
            }

            if !cell.ch.is_whitespace() {
                insert_attrs(
                    cell,
                    &attr_list,
                    byte_offset as u32,
                    (byte_offset + len) as u32,
                );
            }

            byte_offset += len;
        }

        StyledLine {
            line_str,
            cell_to_byte: cell_to_byte.into_boxed_slice(),
            attr_list,
        }
    }
}

fn insert_attrs(cell: &Cell, attr_list: &pango::AttrList, start_idx: u32, end_idx: u32) {
    if cell.attrs.italic {
        let mut attr = pango::Attribute::new_style(pango::Style::Italic).unwrap();
        attr.set_start_index(start_idx);
        attr.set_end_index(end_idx);
        attr_list.insert(attr);
    }
    if cell.attrs.bold {
        let mut attr = pango::Attribute::new_weight(pango::Weight::Bold).unwrap();
        attr.set_start_index(start_idx);
        attr.set_end_index(end_idx);
        attr_list.insert(attr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_styled_line() {
        let mut line = Line::new(3);
        line[0].ch = 'a';
        line[1].ch = 'b';
        line[2].ch = 'c';

        let styled_line = StyledLine::from(&line);
        assert_eq!("abc", styled_line.line_str);
        assert_eq!(3, styled_line.cell_to_byte.len());
        assert_eq!(0, styled_line.cell_to_byte[0]);
        assert_eq!(1, styled_line.cell_to_byte[1]);
        assert_eq!(2, styled_line.cell_to_byte[2]);
    }
}
