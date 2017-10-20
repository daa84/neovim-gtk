use toml;

use settings::SettingsLoader;
use super::vim_plug;

pub struct Store {
    settings: Settings,
}

impl Store {
    pub fn load() -> Self {
        Store { settings: Settings::load() }
    }

    pub fn load_from_plug(vim_plug: &vim_plug::Manager) -> Self {
        let settings = match vim_plug.get_plugs() {
            Err(msg) => {
                error!("{}", msg);
                Settings::empty()
            }
            Ok(plugs) => {
                let plugs = plugs
                    .iter()
                    .map(|vpi| PlugInfo::new(vpi.name.to_owned(), vpi.uri.to_owned()))
                    .collect();
                Settings::new(plugs)
            }
        };

        Store { settings }
    }
}

#[derive(Serialize, Deserialize)]
struct Settings {
    plugs: Vec<PlugInfo>,
}

impl Settings {
    fn new(plugs: Vec<PlugInfo>) -> Self {
        Settings { plugs }
    }
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
