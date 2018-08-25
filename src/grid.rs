use std::collections::HashMap;
use std::ops::Index;

use neovim_lib::Value;

use highlight::HighlightMap;
use ui_model::{UiModel, ModelRect, ModelRectVec};

pub struct GridMap {
    grids: HashMap<u64, Grid>,
}

impl Index<u64> for GridMap {
    type Output = Grid;

    fn index(&self, idx: u64) -> &Grid {
        &self.grids[&idx]
    }
}

impl GridMap {
    pub fn new() -> Self {
        GridMap {
            grids: HashMap::new(),
        }
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
}

impl Grid {
    pub fn new() -> Self {
        Grid {
            model: UiModel::empty(),
        }
    }

    pub fn cursor_goto(&mut self, row: usize, col: usize) -> ModelRectVec {
        self.model.set_cursor(row, col)
    }

    pub fn clear(&mut self) {
        self.model.clear();
    }

    pub fn line(&mut self, row: usize, col_start: usize, cells: Vec<Vec<Value>>, highlights: &HighlightMap) -> ModelRect {
        let starting_hl = None;
        let col_end = col_start;

        for cell in cells {
            let ch = cell.get(0).unwrap().as_str().unwrap_or("");
            let hl_id = cell.get(1).and_then(|h| h.as_u64()).or(starting_hl);
            let repeat = cell.get(2).and_then(|r| r.as_u64()).unwrap_or(1) as usize;

            if starting_hl.is_none() {
                starting_hl = hl_id;
            }

            let put_rect = self.model.put(row, col_end, ch, ch.is_empty(), repeat, highlights.get(hl_id.unwrap()));
            col_end += repeat;
        }


        ModelRect::new(row, row, col_start, col_end - 1)
    }

    pub fn scroll(&mut self, top: u64, bot: u64, left: u64, right: u64, rows: i64, _: i64) -> ModelRect {
        self.model.scroll(top as i64, bot as i64, left as usize, right as usize, rows)
    }
}
