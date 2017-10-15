use std::ops::{Deref, DerefMut};

use neovim_lib::Neovim;

enum NeovimClientState {
    Uninitialized,
    InitInProgress,
    Initialized(Neovim),
    Error,
}

impl NeovimClientState {
    pub fn is_initializing(&self) -> bool {
        match *self {
            NeovimClientState::InitInProgress => true,
            _ => false,
        }
    }

    pub fn is_uninitialized(&self) -> bool {
        match *self {
            NeovimClientState::Uninitialized => true,
            _ => false,
        }
    }

    pub fn is_initialized(&self) -> bool {
        match *self {
            NeovimClientState::Initialized(_) => true,
            _ => false,
        }
    }

    pub fn nvim(&self) -> &Neovim {
        match *self {
            NeovimClientState::Initialized(ref nvim) => nvim,
            NeovimClientState::InitInProgress |
            NeovimClientState::Uninitialized => panic!("Access to uninitialized neovim client"),
            NeovimClientState::Error => {
                panic!("Access to neovim client that is not started due to some error")
            }
        }
    }

    pub fn nvim_mut(&mut self) -> &mut Neovim {
        match *self {
            NeovimClientState::Initialized(ref mut nvim) => nvim,
            NeovimClientState::InitInProgress |
            NeovimClientState::Uninitialized => panic!("Access to uninitialized neovim client"),
            NeovimClientState::Error => {
                panic!("Access to neovim client that is not started due to some error")
            }
        }
    }
}

pub struct NeovimClient {
    state: NeovimClientState,
}

impl NeovimClient {
    pub fn new() -> Self {
        NeovimClient { state: NeovimClientState::Uninitialized }
    }

    pub fn set_initialized(&mut self, nvim: Neovim) {
        self.state = NeovimClientState::Initialized(nvim);
    }

    pub fn set_error(&mut self) {
        self.state = NeovimClientState::Error;
    }

    pub fn set_in_progress(&mut self) {
        self.state = NeovimClientState::InitInProgress;
    }

    pub fn is_initialized(&self) -> bool {
        self.state.is_initialized()
    }

    pub fn is_uninitialized(&self) -> bool {
        self.state.is_uninitialized()
    }

    pub fn is_initializing(&self) -> bool {
        self.state.is_initializing()
    }

    pub fn nvim(&self) -> &Neovim {
        self.state.nvim()
    }

    pub fn nvim_mut(&mut self) -> &mut Neovim {
        self.state.nvim_mut()
    }
}

impl Deref for NeovimClient {
    type Target = Neovim;

    fn deref(&self) -> &Neovim {
        self.nvim()
    }
}

impl DerefMut for NeovimClient {
    fn deref_mut(&mut self) -> &mut Neovim {
        self.nvim_mut()
    }
}

