use std::rc::Rc;
use std::cell::RefCell;

use super::vim_plug;
use super::store::Store;
use nvim::NeovimClient;

pub struct Manager {
    pub vim_plug: vim_plug::Manager,
}

impl Manager {
    pub fn new() -> Self {
        Manager {  
            vim_plug: vim_plug::Manager::new(),
        }
    }

    pub fn initialize(&mut self, nvim: Rc<RefCell<NeovimClient>>) {
        self.vim_plug.initialize(nvim);
    }

    pub fn load_store(&self, vim_plug_state: &vim_plug::State) -> Store {
        match *vim_plug_state {
            vim_plug::State::AlreadyLoaded => {
                let store = Store::load_from_plug(&self.vim_plug);
                store
            }
            vim_plug::State::Unknown => {
                Store::load()
            }
        }
    }
}
