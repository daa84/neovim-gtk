use super::cell::Cell;
use pango::Item;

pub struct Line {
    line: Box<[Cell]>,
    item_line: Option<Box<[Item]>>,
    cell_to_item: Box<[usize]>,
}

impl Line {
    pub fn new(columns: usize) -> Self {
        let line = Vec::with_capacity(columns);
        for _ in 0..columns {
            line.push(Cell::new(' '));
        }

        Line { 
            cell_to_item: Vec::with_capacity(line.len()).into_boxed_slice(),
            line: line.into_boxed_slice(),
            item_line: None,
        }
    }
}
