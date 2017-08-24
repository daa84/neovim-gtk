use std::ops::{Index, IndexMut};

use super::cell::Cell;
use pango;

pub struct Item {
    item: pango::Item,
    glyph_string: Option<pango::GlyphString>,
}

impl Item {
    pub fn new(item: pango::Item) -> Self {
        Item {
            item,
            glyph_string: None,
        }
    }
}

pub struct Line {
    pub line: Box<[Cell]>,
    item_line: Option<Box<[Item]>>,
    cell_to_item: Box<[usize]>,
}

impl Line {
    pub fn new(columns: usize) -> Self {
        let mut line = Vec::with_capacity(columns);
        for _ in 0..columns {
            line.push(Cell::new(' '));
        }

        Line {
            cell_to_item: Vec::with_capacity(line.len()).into_boxed_slice(),
            line: line.into_boxed_slice(),
            item_line: None,
        }
    }

    pub fn clear(&mut self, left: usize, right: usize) {
        for cell in &mut self.line[left..right + 1] {
            cell.clear();
        }

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
            let len = line_str.len();

            for i in byte_offset..byte_offset + len {
                cell_to_byte.push(cell_idx);
            }

            insert_attrs(cell, &attr_list, byte_offset as u32, (byte_offset + len) as u32);

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
