use std::rc::Rc;

use super::vim_plug;
use super::store::{Store, PlugInfo};

use crate::nvim::NeovimClient;

pub struct Manager {
    pub vim_plug: vim_plug::Manager,
    pub store: Store,
    pub plug_manage_state: PlugManageState,
}

impl Manager {
    pub fn new() -> Self {
        let (plug_manage_state, store) = if Store::is_config_exists() {
            (PlugManageState::NvimGtk, Store::load())
        } else {
            (PlugManageState::Unknown, Default::default())
        };

        Manager {
            vim_plug: vim_plug::Manager::new(),
            plug_manage_state,
            store,
        }
    }

    pub fn generate_config(&self) -> Option<PlugManagerConfigSource> {
        if self.store.is_enabled() {
            Some(PlugManagerConfigSource::new(&self.store))
        } else {
            None
        }
    }

    pub fn init_nvim_client(&mut self, nvim: Rc<NeovimClient>) {
        self.vim_plug.initialize(nvim);
    }

    pub fn reload_store(&mut self) {
        match self.plug_manage_state {
            PlugManageState::Unknown => {
                if self.vim_plug.is_loaded() {
                    self.store = Store::load_from_plug(&self.vim_plug);
                    self.plug_manage_state = PlugManageState::VimPlug;
                } else {
                    self.store = Default::default();
                }
            }
            PlugManageState::NvimGtk => {
                if Store::is_config_exists() {
                    self.store = Store::load();
                } else {
                    self.store = Default::default();
                }
            }
            PlugManageState::VimPlug => {
                if Store::is_config_exists() {
                    self.store = Store::load();
                    self.plug_manage_state = PlugManageState::NvimGtk;
                } else {
                    self.store = Default::default();
                }
            }
        }
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

    pub fn add_plug(&mut self, plug: PlugInfo) -> bool {
        self.store.add_plug(plug)
    }

    pub fn move_item(&mut self, idx: usize, offset: i32) {
        self.store.move_item(idx, offset);
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
                builder += &format!(
                    "Plug '{}', {{ 'as': '{}' }}\n",
                    plug.get_plug_path(),
                    plug.name
                );
            }
        }

        builder += "call plug#end()\n";

        PlugManagerConfigSource { source: builder }
    }
}
