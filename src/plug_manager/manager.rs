use std::rc::Rc;
use std::cell::RefCell;

use super::vim_plug;
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
}
