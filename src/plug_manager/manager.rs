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

    pub fn load_config(&mut self) -> Option<PlugManagerConfigSource> {
        if Store::is_config_exists() {
            let store = Store::load();
            if store.is_enabled() {
                let config = PlugManagerConfigSource::new(&store);
                self.plug_manage_state = PlugManageState::NvimGtk(store);
                Some(config)
            } else {
                self.plug_manage_state = PlugManageState::NvimGtk(store);
                None
            }
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
                self.plug_manage_state =
                    PlugManageState::VimPlug(Store::load_from_plug(&self.vim_plug));
            }
        }
    }

    pub fn store_mut(&mut self) -> Option<&mut Store> {
        match self.plug_manage_state {
            PlugManageState::NvimGtk(ref mut store) => Some(store),
            PlugManageState::VimPlug(ref mut store) => Some(store),
            PlugManageState::Unknown => None,
        }
    }

    pub fn store(&self) -> Option<&Store> {
        match self.plug_manage_state {
            PlugManageState::NvimGtk(ref store) => Some(store),
            PlugManageState::VimPlug(ref store) => Some(store),
            PlugManageState::Unknown => None,
        }
    }

    pub fn save(&self) {
        self.store().map(|s| s.save());
    }

    pub fn clear_removed(&mut self) {
        self.store_mut().map(|s| s.clear_removed());
    }
}

pub enum PlugManageState {
    NvimGtk(Store),
    VimPlug(Store),
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
