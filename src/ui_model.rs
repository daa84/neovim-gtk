
#[derive(Clone)]
pub struct Color(pub f64, pub f64, pub f64);

const COLOR_BLACK: Color = Color(0.0, 0.0, 0.0);
const COLOR_WHITE: Color = Color(1.0, 1.0, 1.0);

#[derive(Clone)]
pub struct Attrs {
    pub italic: bool,
    pub bold: bool,
    pub foreground: Color,
    pub background: Color,
}

impl Attrs {
    pub fn new() -> Attrs {
        Attrs {
            foreground: COLOR_WHITE,
            background: COLOR_BLACK,
            italic: false,
            bold: false,
        }
    }

    fn clear(&mut self) {
        self.italic = false;
        self.bold = false;
        self.foreground = COLOR_WHITE;
        self.background = COLOR_BLACK;
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
        self.attrs.clear();
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

    pub fn eol_clear(&mut self) {
        let (cur_row, cur_col, columns) = (self.cur_row, self.cur_col, self.columns);
        self.clear_region(cur_row, cur_row, cur_col, columns - 1);
    }

    fn clear_region(&mut self, top: usize, bot: usize, left: usize, right: usize) {
        for row in top..bot + 1 {
            for col in left..right + 1 {
                self.model[row][col].clear();
            }
        }
    }
}
