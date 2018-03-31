use neovim_lib::Value;

use value::ValueMapExt;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct ModeInfo {
    cursor_shape: Option<CursorShape>,
    cell_percentage: Option<u64>,
}

impl ModeInfo {
    pub fn new(mode_info_arr: &Vec<(Value, Value)>) -> Result<Self, String> {
        let mode_info_map = mode_info_arr.to_attrs_map()?;

        let cursor_shape = if let Some(shape) = mode_info_map.get("cursor_shape") {
            Some(CursorShape::new(shape)?)
        } else {
            None
        };

        let cell_percentage = if let Some(cell_percentage) = mode_info_map.get("cell_percentage") {
            cell_percentage.as_u64()
        } else {
            None
        };

        Ok(ModeInfo {
            cursor_shape,
            cell_percentage,
        })
    }

    pub fn cursor_shape(&self) -> Option<&CursorShape> {
        self.cursor_shape.as_ref()
    }

    pub fn cell_percentage(&self) -> u64 {
        self.cell_percentage.unwrap_or(0)
    }
}
