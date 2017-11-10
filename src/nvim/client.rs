use std::ops::{Deref, DerefMut};
use std::cell::{Cell, RefCell, RefMut};
use std::sync::{Arc, Mutex, MutexGuard};

use neovim_lib::Neovim;

#[derive(Clone, Copy, PartialEq)]
enum NeovimClientState {
    Uninitialized,
    InitInProgress,
    Initialized,
    Error,
}

pub enum NeovimRef<'a> {
    SingleThreaded(RefMut<'a, Neovim>),
    MultiThreaded {
        guard: MutexGuard<'a, RefCell<Option<Neovim>>>,
        nvim: RefMut<'a, Option<Neovim>>,
    },
}

impl<'a> NeovimRef<'a> {
    fn from_nvim(nvim: RefMut<'a, Neovim>) -> Self {
        NeovimRef::SingleThreaded(nvim)
    }

    fn is_some(&self) -> bool {
        match *self {
            NeovimRef::MultiThreaded{ref nvim, ..} => nvim.is_some(),
            NeovimRef::SingleThreaded(_) => true,
        }
    }

    fn from_nvim_async(nvim_async: &'a NeovimClientAsync) -> Option<NeovimRef<'a>> {
        let guard = nvim_async.nvim.lock().unwrap();
        let nvim = guard.borrow_mut();

        let nvim_ref = NeovimRef::MultiThreaded { guard, nvim };

        if nvim_ref.is_some() {
            Some(nvim_ref)
        } else {
            None
        }
    }
}

impl<'a> Deref for NeovimRef<'a> {
    type Target = Neovim;

    fn deref(&self) -> &Neovim {
        match *self {
            NeovimRef::SingleThreaded(ref nvim) => &*nvim,
            NeovimRef::MultiThreaded { ref nvim, .. } => (&*nvim).as_ref().unwrap(),
        }
    }
}

impl<'a> DerefMut for NeovimRef<'a> {
    fn deref_mut(&mut self) -> &mut Neovim {
        match *self {
            NeovimRef::SingleThreaded(ref mut nvim) => &mut *nvim,
            NeovimRef::MultiThreaded { ref mut nvim, .. } => (&mut *nvim).as_mut().unwrap(),
        }
    }
}

pub struct NeovimClientAsync {
    nvim: Arc<Mutex<RefCell<Option<Neovim>>>>,
}

impl NeovimClientAsync {
    fn new(nvim: Neovim) -> Self {
        NeovimClientAsync { nvim: Arc::new(Mutex::new(RefCell::new(Some(nvim)))) }
    }

    pub fn borrow(&self) -> NeovimRef {
        NeovimRef::from_nvim_async(self).unwrap()
    }
}

impl Clone for NeovimClientAsync {
    fn clone(&self) -> Self {
        NeovimClientAsync {
            nvim: self.nvim.clone()
        }
    }
}

pub struct NeovimClient {
    state: Cell<NeovimClientState>,
    nvim: RefCell<Option<Neovim>>,
    nvim_async: RefCell<Option<NeovimClientAsync>>,
}

impl NeovimClient {
    pub fn new() -> Self {
        NeovimClient {
            state: Cell::new(NeovimClientState::Uninitialized),
            nvim: RefCell::new(None),
            nvim_async: RefCell::new(None),
        }
    }

    pub fn async_to_sync(&self) {
        {
            let lock = self.nvim_async
                .borrow()
                .as_ref()
                .expect("Nvim not initialized")
                .nvim
                .lock()
                .unwrap();
            let nvim = lock.borrow_mut().take().unwrap();
            *self.nvim.borrow_mut() = Some(nvim);
        }
        *self.nvim_async.borrow_mut() = None;
    }

    pub fn set_nvim_async(&self, nvim: Neovim) -> NeovimClientAsync {
        let nvim_async = NeovimClientAsync::new(nvim);
        *self.nvim_async.borrow_mut() = Some(nvim_async.clone());
        nvim_async
    }

    pub fn set_initialized(&self) {
        self.state.set(NeovimClientState::Initialized);
    }

    pub fn set_error(&self) {
        self.state.set(NeovimClientState::Error);
    }

    pub fn set_in_progress(&self) {
        self.state.set(NeovimClientState::InitInProgress);
    }

    pub fn is_initialized(&self) -> bool {
        self.state.get() == NeovimClientState::Initialized
    }

    pub fn is_uninitialized(&self) -> bool {
        self.state.get() == NeovimClientState::Uninitialized
    }

    pub fn is_initializing(&self) -> bool {
        self.state.get() == NeovimClientState::InitInProgress
    }

    pub fn nvim(&self) -> Option<NeovimRef> {
        let nvim = self.nvim.borrow_mut();
        if nvim.is_some() {
            Some(NeovimRef::from_nvim(
                RefMut::map(nvim, |n| n.as_mut().unwrap()),
            ))
        } else {
            let nvim_async = self.nvim_async.borrow();
            if let Some(ref nvim_async) = *nvim_async {
                NeovimRef::from_nvim_async(nvim_async)
            } else {
                None
            }
        }
    }
}
