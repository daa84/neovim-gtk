use std::rc::Rc;
use std::cell::RefCell;

use super::vim_plug;
use super::store::{Store, PlugInfo};

use nvim::NeovimClient;

pub struct Manager {
    vim_plug: vim_plug::Manager,
    pub store: Store,
    pub plug_manage_state: PlugManageState,
}

impl Manager {
    pub fn new() -> Self {

        let (plug_manage_state, store) = if Store::is_config_exists() {
            (PlugManageState::NvimGtk, Store::load())
        } else {
            (PlugManageState::Unknown, Store::empty())
        };

        Manager {
            vim_plug: vim_plug::Manager::new(),
            plug_manage_state,
            store,
        }
    }

    pub fn load_config(&self) -> Option<PlugManagerConfigSource> {
        if self.store.is_enabled() {
            Some(PlugManagerConfigSource::new(&self.store))
        } else {
            None
        }
    }

    pub fn init_nvim_client(&mut self, nvim: Rc<RefCell<NeovimClient>>) {
        self.vim_plug.initialize(nvim);
    }

    pub fn update_state(&mut self) {
        if let PlugManageState::Unknown = self.plug_manage_state {
            if self.vim_plug.is_loaded() {
                self.store = Store::load_from_plug(&self.vim_plug);
                self.plug_manage_state = PlugManageState::VimPlug;
            }
        }
    }

    pub fn save(&self) {
        self.store.save();
    }

    pub fn clear_removed(&mut self) {
        self.store.clear_removed();
    }

    pub fn add_plug(&mut self, plug: PlugInfo) {
        self.store.add_plug(plug);
    }
}

pub enum PlugManageState {
    NvimGtk,
    VimPlug,
    Unknown,
}

#[derive(Clone)]
pub struct PlugManagerConfigSource {
    pub source: String,
}

impl PlugManagerConfigSource {
    pub fn new(store: &Store) -> Self {
        let mut builder = "call plug#begin()\n".to_owned();

        for plug in store.get_plugs() {
            if !plug.removed {
                builder += &format!("Plug '{}'\n", plug.get_plug_path());
            }
        }

        builder += "call plug#end()\n";

        PlugManagerConfigSource { source: builder }
    }
}
