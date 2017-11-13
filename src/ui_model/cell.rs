use color::Color;

#[derive(Clone)]
pub struct Attrs {
    pub italic: bool,
    pub bold: bool,
    pub underline: bool,
    pub undercurl: bool,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub special: Option<Color>,
    pub reverse: bool,
    pub double_width: bool,
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
            reverse: false,
            double_width: false,
        }
    }

    fn clear(&mut self) {
        self.italic = false;
        self.bold = false;
        self.underline = false;
        self.undercurl = false;
        self.reverse = false;
        self.foreground = None;
        self.background = None;
        self.special = None;
        self.double_width = false;
    }
}

#[derive(Clone)]
pub struct Cell {
    pub attrs: Attrs,
    pub ch: char,
    pub dirty: bool,
}

impl Cell {
    pub fn new(ch: char) -> Cell {
        Cell {
            attrs: Attrs::new(),
            ch: ch,
            dirty: true,
        }
    }

    pub fn clear(&mut self) {
        self.ch = ' ';
        self.attrs.clear();
        self.dirty = true;
    }
}
