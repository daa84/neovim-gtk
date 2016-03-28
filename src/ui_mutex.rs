
use std::cell::{RefCell, RefMut};
use std::thread;

pub struct UiMutex<T: Sized> {
    data: RefCell<T>,
    main_thread_name: Option<String>,
}

// here sync used to mark that internal data is acessed only from main thread
// this behaviour works because of borrow_mut check thread name
unsafe impl<T: Sized + Send> Sync for UiMutex<T> {}

impl<T> UiMutex<T> {
    pub fn new(t: T) -> UiMutex<T> {
        UiMutex {
            data: RefCell::new(t),
            main_thread_name: thread::current().name().map(|v| v.to_owned()),
        }
    }

    pub fn borrow_mut(&self) -> RefMut<T> {
        let current_thread_name = thread::current().name().map(|v| v.to_owned());
        if current_thread_name != self.main_thread_name {
            panic!("Can access value only from main thread, {:?}/{:?}", current_thread_name, self.main_thread_name);
        }

        self.data.borrow_mut()
    }
}


