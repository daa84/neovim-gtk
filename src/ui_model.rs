pub struct Cell {
    ch: char,
}

impl Cell {
    pub fn new(ch: char) -> Cell {
        Cell { ch: ch }
    }
}

pub struct UiModel {
    columns: u64,
    rows: u64,
    cur_row: u64,
    cur_col: u64,
    model: Vec<Cell>,
}

impl UiModel {
    pub fn new(columns: u64, rows: u64) -> UiModel {
        let cells = (columns * rows) as usize;
        let mut model = Vec::with_capacity(cells);
        for i in 0..cells {
            model[i] = Cell::new(' ');
        }

        UiModel { 
            columns: columns,
            rows: rows,
            cur_row: 0,
            cur_col: 0,
            model: model,
        }
    }

    pub fn set_cursor(&mut self, col: u64, row: u64) {
        self.cur_col = col;
        self.cur_row = row;
    }
}
