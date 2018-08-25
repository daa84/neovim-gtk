use std::collections::HashMap;
use std::rc::Rc;

use color::*;
use neovim_lib::Value;

const DEFAULT_HL: u64 = 0;

pub struct HighlightMap {
    highlights: HashMap<u64, Rc<Highlight>>,
    bg_color: Color,
    fg_color: Color,
    sp_color: Color,
}

impl HighlightMap {
    pub fn new() -> Self {
        HighlightMap {
            highlights: HashMap::new(),
            bg_color: COLOR_BLACK,
            fg_color: COLOR_WHITE,
            sp_color: COLOR_RED,
        }
    }

    pub fn set_defaults(&mut self, fg: Color, bg: Color, sp: Color) {
        self.fg_color = fg;
        self.bg_color = bg;
        self.sp_color = sp;
    }

    pub fn get(&self, idx: u64) -> Rc<Highlight> {
        self.highlights.get(&idx).map(Rc::clone).unwrap_or_else(|| {
            self.highlights
                .get(&0)
                .map(Rc::clone)
                .unwrap_or_else(|| Rc::new(Highlight::new()))
        })
    }
}

pub struct Highlight {
    pub italic: bool,
    pub bold: bool,
    pub underline: bool,
    pub undercurl: bool,
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub special: Option<Color>,
    pub reverse: bool,
}

impl Highlight {
    pub fn new() -> Self {
        Highlight {
            foreground: None,
            background: None,
            special: None,
            italic: false,
            bold: false,
            underline: false,
            undercurl: false,
            reverse: false,
        }
    }

    pub fn from_value_map(attrs: &HashMap<String, Value>) -> Self {
        let mut model_attrs = Highlight::new();

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
    }
}
