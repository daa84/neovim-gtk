use nvim;

#[derive(PartialEq)]
pub enum NvimMode {
    Normal,
    Insert,
    Other,
}

pub struct Mode {
    mode: NvimMode,
    idx: usize,
    info: Option<Vec<nvim::ModeInfo>>,
}

impl Mode {
    pub fn new() -> Self {
        Mode {
            mode: NvimMode::Normal,
            idx: 0,
            info: None,
        }
    }

    pub fn is(&self, mode: &NvimMode) -> bool {
        self.mode == *mode
    }

    pub fn update(&mut self, mode: &str, idx: usize) {
        match mode {
            "normal" => self.mode = NvimMode::Normal,
            "insert" => self.mode = NvimMode::Insert,
            _ => self.mode = NvimMode::Other,
        }

        self.idx = idx;
    }

    pub fn set_info(&mut self, cursor_style_enabled: bool, info: Vec<nvim::ModeInfo>) {
        self.info = if cursor_style_enabled {
            Some(info)
        } else {
            None
        };
    }
}
