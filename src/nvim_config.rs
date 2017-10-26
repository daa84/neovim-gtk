use std;
use std::fs::File;
use std::io::Write;

use tempfile;
use plug_manager;

#[derive(Clone)]
pub struct NvimConfig {
    plug_config: Option<plug_manager::PlugManagerConfigSource>,
}

impl NvimConfig {
    pub fn new(plug_config: Option<plug_manager::PlugManagerConfigSource>) -> Self {
        NvimConfig { plug_config }
    }

    pub fn generate_config(&self) -> Option<tempfile::NamedTempFile> {
        if self.plug_config.is_some() {
            match self.write_file() {
                Err(err) => {
                    error!("{}", err);
                    None
                }
                Ok(file) => Some(file),
            }
        } else {
            None
        }
    }

    fn write_file(&self) -> std::io::Result<tempfile::NamedTempFile> {
        let temp_file = tempfile::NamedTempFile::new()?;
        {
            let mut file: &File = &temp_file;
            let content = &self.plug_config.as_ref().unwrap().source;
            if !content.is_empty() {
                file.write_all(content.as_bytes())?;
            }

            file.sync_data()?;
        }
        Ok(temp_file)
    }
}
