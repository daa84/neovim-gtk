use std::collections::HashMap;

use color::Color;
use neovim_lib::Value;

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

    pub fn from_value_map(attrs: &HashMap<String, Value>) -> Attrs {
        let mut model_attrs = Attrs::new();

        for (ref key, ref val) in attrs {
            match key.as_ref() {
                "foreground" => {
                    if let Some(fg) = val.as_u64() {
                        model_attrs.foreground = Some(Color::from_indexed_color(fg));
                    }
                }
                "background" => {
                    if let Some(bg) = val.as_u64() {
                        model_attrs.background = Some(Color::from_indexed_color(bg));
                    }
                }
                "special" => {
                    if let Some(bg) = val.as_u64() {
                        model_attrs.special = Some(Color::from_indexed_color(bg));
                    }
                }
                "reverse" => model_attrs.reverse = true,
                "bold" => model_attrs.bold = true,
                "italic" => model_attrs.italic = true,
                "underline" => model_attrs.underline = true,
                "undercurl" => model_attrs.undercurl = true,
                attr_key => error!("unknown attribute {}", attr_key),
            };
        }

        model_attrs
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

const EMPTY_STRING: String = String::new();

#[derive(Clone)]
pub struct Cell {
    pub attrs: Attrs,
    pub ch: String,
    pub dirty: bool,
}

impl Cell {
    pub fn new_empty() -> Cell {
        Cell::new(EMPTY_STRING)
    }

    pub fn new(ch: String) -> Cell {
        Cell {
            attrs: Attrs::new(),
            ch,
            dirty: true,
        }
    }

    pub fn clear(&mut self) {
        self.ch = EMPTY_STRING;
        self.attrs.clear();
        self.dirty = true;
    }
}
