
#[derive(Clone)]
pub struct Color(pub f64, pub f64, pub f64);

const COLOR_BLACK: Color = Color(0.0, 0.0, 0.0);
const COLOR_WHITE: Color = Color(1.0, 1.0, 1.0);

#[derive(Clone)]
pub struct Attrs {
    pub foreground: Color,
    pub background: Color,
}

impl Attrs {
    pub fn new() -> Attrs {
        Attrs {
            foreground: COLOR_WHITE,
            background: COLOR_BLACK,
        }
    }
}

pub struct Cell {
    pub ch: char,
    pub attrs: Attrs,
}

impl Cell {
    pub fn new(ch: char) -> Cell {
        Cell {
            ch: ch,
            attrs: Attrs::new(),
        }
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
            for _ in 0..columns as usize {
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

    pub fn model(&self) -> &Vec<Vec<Cell>> {
        &self.model
    }

    pub fn set_cursor(&mut self, row: u64, col: u64) {
        self.cur_col = col as usize;
        self.cur_row = row as usize;
    }

    pub fn put(&mut self, text: &str, attrs: &Option<Attrs>) {
        let mut cell = &mut self.model[self.cur_row][self.cur_col];
        cell.ch = text.chars().last().unwrap();
        cell.attrs = attrs.as_ref().map(|o| o.clone()).unwrap_or_else(|| Attrs::new());
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
