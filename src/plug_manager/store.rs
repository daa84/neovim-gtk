use toml;

use settings::SettingsLoader;

pub struct Store {}

impl Store {
    pub fn new() -> Self {
        Store {}
    }
}

#[derive(Serialize, Deserialize)]
struct Settings {
    plugs: Vec<PlugInfo>,
}

impl SettingsLoader for Settings {
    const SETTINGS_FILE: &'static str = "plugs.toml";

    fn empty() -> Self {
        Settings { plugs: vec![] }
    }

    fn from_str(s: &str) -> Result<Self, String> {
        toml::from_str(&s).map_err(|e| format!("{}", e))
    }
}

#[derive(Serialize, Deserialize)]
pub struct PlugInfo {
    name: String,
    url: String,
}

impl PlugInfo {
    pub fn new(name: String, url: String) -> Self {
        PlugInfo { name, url }
    }
}
