pub struct Cell {
    ch: char,
}

impl Cell {
    pub fn new(ch: char) -> Cell {
        Cell { ch: ch }
    }

    fn clear(&mut self) {
         self.ch = ' ';
    }
}

pub struct UiModel {
    columns: usize,
    rows: usize,
    cur_row: usize,
    cur_col: usize,
    model: Vec<Vec<Cell>>,
}

impl UiModel {
    pub fn empty() -> UiModel {
        UiModel::new(0, 0)
    }

    pub fn new(rows: u64, columns: u64) -> UiModel {
        let mut model = Vec::with_capacity(rows as usize);
        for i in 0..rows as usize {
            model.push(Vec::with_capacity(columns as usize));
            for _ in 0..columns as usize{
                model[i].push(Cell::new(' '));
            }
        }

        UiModel { 
            columns: columns as usize,
            rows: rows as usize,
            cur_row: 0,
            cur_col: 0,
            model: model,
        }
    }

    pub fn set_cursor(&mut self, row: u64, col: u64) {
        self.cur_col = col as usize;
        self.cur_row = row as usize;
    }

    pub fn put(&mut self, text: &str) {
        self.model[self.cur_row][self.cur_col].ch = text.chars().last().unwrap();
        self.cur_col += 1;
    }

    pub fn clear(&mut self) {
        for row in 0..self.rows {
            for col in 0..self.columns {
                self.model[row][col].clear();
            }
        }
    }
}
