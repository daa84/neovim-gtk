use std::collections::HashMap;
use neovim_lib::Value;

pub trait ValueMapExt {
    fn to_attrs_map(&self) -> Result<HashMap<&str, &Value>, String>;

    fn to_attrs_map_report(&self) -> Option<HashMap<&str, &Value>>;
}

impl ValueMapExt for Vec<(Value, Value)> {
    fn to_attrs_map(&self) -> Result<HashMap<&str, &Value>, String> {
        self.iter()
            .map(|p| {
                p.0
                    .as_str()
                    .ok_or_else(|| "Can't convert map key to string".to_owned())
                    .map(|key| (key, &p.1))
            })
            .collect::<Result<HashMap<&str, &Value>, String>>()

    }

    fn to_attrs_map_report(&self) -> Option<HashMap<&str, &Value>> {
        match self.to_attrs_map() {
            Err(e) => {
                error!("{}", e);
                None
            }
            Ok(m) => Some(m),
        }
    }
}
