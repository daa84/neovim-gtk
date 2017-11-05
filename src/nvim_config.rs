use std::path::PathBuf;
use std::fs::{remove_file, OpenOptions};
use std::io::Write;

use dirs;
use plug_manager;

#[derive(Clone)]
pub struct NvimConfig {
    plug_config: Option<plug_manager::PlugManagerConfigSource>,
}

impl NvimConfig {
    const CONFIG_PATH: &'static str = "settings.vim";

    pub fn new(plug_config: Option<plug_manager::PlugManagerConfigSource>) -> Self {
        NvimConfig { plug_config }
    }

    pub fn generate_config(&self) -> Option<PathBuf> {
        if self.plug_config.is_some() {
            match self.write_file() {
                Err(err) => {
                    error!("{}", err);
                    None
                }
                Ok(file) => Some(file),
            }
        } else {
            NvimConfig::config_path().map(remove_file);
            None
        }
    }

    pub fn config_path() -> Option<PathBuf> {
        if let Ok(mut path) = dirs::get_app_config_dir() {
            path.push(NvimConfig::CONFIG_PATH);
            if path.is_file() {
                return Some(path);
            }
        }

        None
    }

    fn write_file(&self) -> Result<PathBuf, String> {
        let mut config_dir = dirs::get_app_config_dir_create()?;
        config_dir.push(NvimConfig::CONFIG_PATH);

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&config_dir)
            .map_err(|e| format!("{}", e))?;

        let content = &self.plug_config.as_ref().unwrap().source;
        if !content.is_empty() {
            debug!("{}", content);
            file.write_all(content.as_bytes()).map_err(
                |e| format!("{}", e),
            )?;
        }

        file.sync_all().map_err(|e| format!("{}", e))?;
        Ok(config_dir)
    }
}
