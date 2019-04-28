use std::rc::Rc;

use crate::highlight::Highlight;

#[derive(Clone)]
pub struct Cell {
    pub hl: Rc<Highlight>,
    pub ch: String,
    pub dirty: bool,
    pub double_width: bool,
}

impl Cell {
    pub fn new_empty() -> Cell {
        Cell {
            hl: Rc::new(Highlight::new()),
            ch: String::new(),
            dirty: true,
            double_width: false,
        }
    }

    pub fn clear(&mut self, hl: Rc<Highlight>) {
        self.ch.clear();
        self.hl = hl;
        self.dirty = true;
        self.double_width = false;
    }
}
