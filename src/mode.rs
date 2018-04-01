use std::collections::HashMap;
use neovim_lib::Value;

#[derive(Clone, PartialEq)]
pub enum NvimMode {
    Normal,
    Insert,
    Other,
}

pub struct Mode {
    mode: NvimMode,
    idx: usize,
    info: Option<Vec<ModeInfo>>,
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

    pub fn mode_info(&self) -> Option<&ModeInfo> {
        self.info.as_ref().and_then(|i| i.get(self.idx))
    }

    pub fn update(&mut self, mode: &str, idx: usize) {
        match mode {
            "normal" => self.mode = NvimMode::Normal,
            "insert" => self.mode = NvimMode::Insert,
            _ => self.mode = NvimMode::Other,
        }

        self.idx = idx;
    }

    pub fn set_info(&mut self, cursor_style_enabled: bool, info: Vec<ModeInfo>) {
        self.info = if cursor_style_enabled {
            Some(info)
        } else {
            None
        };
    }
}


#[derive(Debug, PartialEq, Clone)]
pub enum CursorShape {
    Block,
    Horizontal,
    Vertical,
    Unknown,
}

impl CursorShape {
    fn new(shape_code: &Value) -> Result<CursorShape, String> {
        let str_code = shape_code
            .as_str()
            .ok_or_else(|| "Can't convert cursor shape to string".to_owned())?;

        Ok(match str_code {
            "block" => CursorShape::Block,
            "horizontal" => CursorShape::Horizontal,
            "vertical" => CursorShape::Vertical,
            _ => {
                error!("Unknown cursor_shape {}", str_code);
                CursorShape::Unknown
            }
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ModeInfo {
    cursor_shape: Option<CursorShape>,
    cell_percentage: Option<u64>,
    pub blinkwait: Option<u32>,
}

impl ModeInfo {
    pub fn new(mode_info_map: &HashMap<String, Value>) -> Result<Self, String> {
        let cursor_shape = if let Some(shape) = mode_info_map.get("cursor_shape") {
            Some(CursorShape::new(shape)?)
        } else {
            None
        };

        Ok(ModeInfo {
            cursor_shape,
            cell_percentage: mode_info_map.get("cell_percentage").and_then(|cp| cp.as_u64()),
            blinkwait: mode_info_map.get("blinkwait").and_then(|cp| cp.as_u64()).map(|v| v as u32),
        })
    }

    pub fn cursor_shape(&self) -> Option<&CursorShape> {
        self.cursor_shape.as_ref()
    }

    pub fn cell_percentage(&self) -> u64 {
        self.cell_percentage.unwrap_or(0)
    }
}
