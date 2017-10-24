use std::rc::Rc;
use std::cell::RefCell;

use super::vim_plug;
use super::store::Store;
use nvim::NeovimClient;

pub struct Manager {
    vim_plug: vim_plug::Manager,
    pub plug_manage_state: PlugManageState,
}

impl Manager {
    pub fn new() -> Self {
        Manager {  
            vim_plug: vim_plug::Manager::new(),
            plug_manage_state: PlugManageState::Unknown,
        }
    }

    pub fn generate_plug_config(&mut self) -> Option<String> {
        if Store::is_config_exists() {
            self.plug_manage_state = PlugManageState::NvimGtk(Store::load());
            Some("TODO".to_owned())
        } else {
            None
        }
    }

    pub fn init_nvim_client(&mut self, nvim: Rc<RefCell<NeovimClient>>) {
        self.vim_plug.initialize(nvim);
    }

    pub fn update_state(&mut self) {
        if self.vim_plug.is_loaded() {
            if let PlugManageState::Unknown = self.plug_manage_state {
                self.plug_manage_state = PlugManageState::Configuration(Store::load_from_plug(&self.vim_plug));
            }
        }
    }
}

pub enum PlugManageState {
    NvimGtk(Store),
    Configuration(Store),
    Unknown,
}
