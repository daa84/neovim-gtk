use std::ops::{Index, IndexMut};
use std::rc::Rc;

use gtk::{self, prelude::*};

use fnv::FnvHashMap;

use neovim_lib::Value;

use highlight::{Highlight, HighlightMap};
use ui_model::{ModelRect, ModelRectVec, UiModel};

const DEFAULT_GRID: u64 = 1;

pub struct GridMap {
    grids: FnvHashMap<u64, Grid>,
}

impl Index<u64> for GridMap {
    type Output = Grid;

    fn index(&self, idx: u64) -> &Grid {
        &self.grids[&idx]
    }
}

impl IndexMut<u64> for GridMap {
    fn index_mut(&mut self, idx: u64) -> &mut Grid {
        self.grids.get_mut(&idx).unwrap()
    }
}

impl GridMap {
    pub fn new() -> Self {
        GridMap {
            grids: FnvHashMap::default(),
        }
    }

    pub fn current(&self) -> Option<&Grid> {
        self.grids.get(&DEFAULT_GRID)
    }

    pub fn current_model_mut(&mut self) -> Option<&mut UiModel> {
        self.grids.get_mut(&DEFAULT_GRID).map(|g| &mut g.model)
    }

    pub fn current_model(&self) -> Option<&UiModel> {
        self.grids.get(&DEFAULT_GRID).map(|g| &g.model)
    }

    pub fn get_or_create(&mut self, idx: u64) -> &mut Grid {
        if self.grids.contains_key(&idx) {
            return self.grids.get_mut(&idx).unwrap();
        }

        self.grids.insert(idx, Grid::new());
        self.grids.get_mut(&idx).unwrap()
    }

    pub fn destroy(&mut self, idx: u64) {
        self.grids.remove(&idx);
    }

    pub fn clear_glyphs(&mut self) {
        for grid in self.grids.values_mut() {
            grid.model.clear_glyphs();
        }
    }
}

pub struct Grid {
    model: UiModel,
    drawing_area: gtk::DrawingArea,
}

impl Grid {
    pub fn new() -> Self {
        Grid {
            model: UiModel::empty(),
            drawing_area: gtk::DrawingArea::new(),
        }
    }

    pub fn get_cursor(&self) -> (usize, usize) {
        self.model.get_cursor()
    }

    pub fn cur_point(&self) -> ModelRect {
        self.model.cur_point()
    }

    pub fn resize(&mut self, columns: u64, rows: u64) {
        if self.model.columns != columns as usize || self.model.rows != rows as usize {
            self.model = UiModel::new(rows, columns);
        }
    }

    pub fn cursor_goto(&mut self, row: usize, col: usize) -> ModelRectVec {
        self.model.set_cursor(row, col)
    }

    pub fn clear(&mut self, default_hl: &Rc<Highlight>) {
        self.model.clear(default_hl);
    }

    pub fn line(
        &mut self,
        row: usize,
        col_start: usize,
        cells: Vec<Vec<Value>>,
        highlights: &HighlightMap,
    ) -> ModelRect {
        let mut hl_id = None;
        let mut col_end = col_start;

        for cell in cells {
            let ch = cell.get(0).unwrap().as_str().unwrap_or("");
            hl_id = cell.get(1).and_then(|h| h.as_u64()).or(hl_id);
            let repeat = cell.get(2).and_then(|r| r.as_u64()).unwrap_or(1) as usize;

            self.model.put(
                row,
                col_end,
                ch,
                ch.is_empty(),
                repeat,
                highlights.get(hl_id.unwrap()),
            );
            col_end += repeat;
        }

        ModelRect::new(row, row, col_start, col_end - 1)
    }

    pub fn scroll(
        &mut self,
        top: u64,
        bot: u64,
        left: u64,
        right: u64,
        rows: i64,
        _: i64,
        default_hl: &Rc<Highlight>,
    ) -> ModelRect {
        self.model.scroll(
            top as i64,
            bot as i64 - 1,
            left as usize,
            right as usize - 1,
            rows,
            default_hl,
        )
    }
}
