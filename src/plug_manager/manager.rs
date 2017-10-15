use std::rc::Rc;
use std::cell::RefCell;

use nvim::NeovimClient;

pub struct Manager {
    
}

impl Manager {
    pub fn new() -> Self {
        Manager {  }
    }

    pub fn initialize(&mut self, nvim: Rc<RefCell<NeovimClient>>) {
    }
}
