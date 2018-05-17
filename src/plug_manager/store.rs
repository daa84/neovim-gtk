use toml;

use settings::SettingsLoader;
use super::vim_plug;

#[derive(Default)]
pub struct Store {
    settings: Settings,
}

impl Store {
    pub fn is_config_exists() -> bool {
        Settings::is_file_exists()
    }

    pub fn is_enabled(&self) -> bool {
        self.settings.enabled
    }

    pub fn load() -> Self {
        Store { settings: Settings::load() }
    }

    pub fn load_from_plug(vim_plug: &vim_plug::Manager) -> Self {
        let settings = match vim_plug.get_plugs() {
            Err(msg) => {
                error!("{}", msg);
                Default::default()
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

    pub fn get_plugs(&self) -> &[PlugInfo] {
        &self.settings.plugs
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.settings.enabled = enabled;
    }

    pub fn clear_removed(&mut self) {
        self.settings.plugs.retain(|p| !p.removed);
    }

    pub fn save(&self) {
        self.settings.save();
    }

    pub fn remove_plug(&mut self, idx: usize) {
        self.settings.plugs[idx].removed = true;
    }

    pub fn restore_plug(&mut self, idx: usize) {
        self.settings.plugs[idx].removed = false;
    }

    pub fn add_plug(&mut self, plug: PlugInfo) -> bool {
        let path = plug.get_plug_path();
        if self.settings.plugs.iter().any(|p| {
            p.get_plug_path() == path || p.name == plug.name
        })
        {
            return false;
        }
        self.settings.plugs.push(plug);
        true
    }

    pub fn plugs_count(&self) -> usize {
        self.settings.plugs.len()
    }

    pub fn move_item(&mut self, idx: usize, offset: i32) {
        let plug = self.settings.plugs.remove(idx);
        self.settings.plugs.insert(
            (idx as i32 + offset) as usize,
            plug,
        );
    }
}

#[derive(Serialize, Deserialize)]
struct Settings {
    enabled: bool,
    plugs: Vec<PlugInfo>,
}

impl Settings {
    fn new(plugs: Vec<PlugInfo>) -> Self {
        Settings {
            plugs,
            enabled: false,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            plugs: vec![],
            enabled: false,
        }
    }
}

impl SettingsLoader for Settings {
    const SETTINGS_FILE: &'static str = "plugs.toml";

    fn from_str(s: &str) -> Result<Self, String> {
        toml::from_str(&s).map_err(|e| format!("{}", e))
    }
}

#[derive(Serialize, Deserialize)]
pub struct PlugInfo {
    pub name: String,
    pub url: String,
    pub removed: bool,
}

impl PlugInfo {
    pub fn new(name: String, url: String) -> Self {
        PlugInfo {
            name,
            url,
            removed: false,
        }
    }

    pub fn get_plug_path(&self) -> String {
        if self.url.contains("github.com") {
            let mut path_comps: Vec<&str> = self.url
                .trim_right_matches(".git")
                .rsplit('/')
                .take(2)
                .collect();
            path_comps.reverse();
            path_comps.join("/")
        } else {
            self.url.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_plug_path() {
        let plug = PlugInfo::new(
            "rust.vim".to_owned(),
            "https://git::@github.com/rust-lang/rust.vim.git".to_owned(),
        );
        assert_eq!("rust-lang/rust.vim".to_owned(), plug.get_plug_path());
    }
}
