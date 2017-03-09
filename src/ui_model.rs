
#[derive(Clone)]
pub struct Color(pub f64, pub f64, pub f64);

pub const COLOR_BLACK: Color = Color(0.0, 0.0, 0.0);
pub const COLOR_WHITE: Color = Color(1.0, 1.0, 1.0);
pub const COLOR_RED: Color = Color(1.0, 0.0, 0.0);

#[derive(Clone)]
pub struct Attrs {
    pub italic: bool,
    pub bold: bool,
    pub underline: bool,
    pub undercurl: bool,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub special: Option<Color>,
}

impl Attrs {
    pub fn new() -> Attrs {
        Attrs {
            foreground: None,
            background: None,
            special: None,
            italic: false,
            bold: false,
            underline: false,
            undercurl: false,
        }
    }

    fn clear(&mut self) {
        self.italic = false;
        self.bold = false;
        self.underline = false;
        self.undercurl = false;
        self.foreground = None;
        self.background = None;
        self.special = None;
    }
}

#[derive(Clone)]
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
    pub columns: usize,
    pub rows: usize,
    cur_row: usize,
    cur_col: usize,
    model: Vec<Vec<Cell>>,
    top: usize,
    bot: usize,
    left: usize,
    right: usize,
}

impl UiModel {
    pub fn empty() -> UiModel {
        UiModel {
            columns: 0,
            rows: 0,
            cur_row: 0,
            cur_col: 0,
            model: vec![],
            top: 0,
            bot: 0,
            left: 0,
            right: 0,
        }
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
            top: 0,
            bot: (rows - 1) as usize,
            left: 0,
            right: (columns - 1) as usize,
        }
    }

    pub fn model(&self) -> &Vec<Vec<Cell>> {
        &self.model
    }

    pub fn set_cursor(&mut self, row: u64, col: u64) {
        self.cur_row = row as usize;
        self.cur_col = col as usize;
    }

    pub fn get_cursor(&self) -> (usize, usize) {
        (self.cur_row, self.cur_col)
    }

    pub fn put(&mut self, text: &str, attrs: Option<&Attrs>) {
        let mut cell = &mut self.model[self.cur_row][self.cur_col];
        cell.ch = text.chars().last().unwrap();
        cell.attrs = attrs.map(Attrs::clone).unwrap_or_else(|| Attrs::new());
        self.cur_col += 1;
    }

    pub fn set_scroll_region(&mut self, top: u64, bot: u64, left: u64, right: u64) {
        self.top = top as usize;
        self.bot = bot as usize;
        self.left = left as usize;
        self.right = right as usize;
    }

    fn copy_row(&mut self, row: usize, offset: i64, left: usize, right: usize) {
        for col in left..right + 1 {
            let from_row = (row as i64 + offset) as usize;
            let from_cell = self.model[from_row][col].clone();
            self.model[row][col] = from_cell;
        }
    }

    pub fn scroll(&mut self, count: i64) {
        let (top, bot, left, right) = (self.top as i64, self.bot as i64, self.left, self.right);

        if count > 0 {
            for row in top as usize..(bot - count + 1) as usize {
                self.copy_row(row, count, left, right);
            }
        } else {
            for row in ((top - count) as usize..(bot + 1) as usize).rev() {
                self.copy_row(row, count, left, right);
            }
        }

        if count > 0 {
            self.clear_region((bot - count + 1) as usize, bot as usize, left, right);
        } else {
            self.clear_region(top as usize, (top - count - 1) as usize, left, right);
        }
    }

    pub fn clear(&mut self) {
        let (rows, columns) = (self.rows, self.columns);
        self.clear_region(0, rows - 1, 0, columns - 1);
    }

    pub fn eol_clear(&mut self) {
        let (cur_row, cur_col, columns) = (self.cur_row, self.cur_col, self.columns);
        self.clear_region(cur_row, cur_row, cur_col, columns - 1);
    }

    fn clear_region(&mut self, top: usize, bot: usize, left: usize, right: usize) {
        for row in &mut self.model[top..bot + 1] {
            for cell in &mut row[left..right + 1] {
                cell.clear();
            }
        }
    }
}
