use std::rc::Rc;
use std::cell::{RefCell, RefMut};

use neovim_lib::{Neovim, NeovimApi, Value};

use nvim::NeovimClient;

pub struct Manager {
   nvim: Option<Rc<RefCell<NeovimClient>>>,
}

impl Manager {
    pub fn new() -> Self {
        Manager { 
            nvim: None,
        }
    }

    pub fn initialize(&mut self, nvim: Rc<RefCell<NeovimClient>>) {
        self.nvim = Some(nvim);
    }

    fn nvim(&self) -> Option<RefMut<Neovim>> {
        let nvim_client = self.nvim.as_ref().unwrap();
        if nvim_client.borrow().is_initialized() {
            Some(RefMut::map(nvim_client.borrow_mut(), |n| n.nvim_mut()))
        } else {
            None
        }
    }

    pub fn get_state(&self) -> State {
        if let Some(mut nvim) = self.nvim() {
            nvim.command("exists('g:loaded_plug')");
        }
        
        State::Unknown
    }
}

pub enum State {
    Unknown,
    AlreadyLoaded,
}
