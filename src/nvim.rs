use neovim_lib::{Neovim, NeovimApi, Session};
use std::io::{Result, Error, ErrorKind};
use std::cell::UnsafeCell;
use std::thread;
use std::sync::Arc;
use rmp::Value;
use ui::Ui;
use gtk;

pub struct MainLoopMutex<T: Sized> {
    data: UnsafeCell<T>,
    main_thread_name: Option<String>,
}

unsafe impl<T: Sized + Send> Sync for MainLoopMutex<T> {}

impl<T> MainLoopMutex<T> {
    pub fn new(t: T) -> MainLoopMutex<T> {
        MainLoopMutex {
            data: UnsafeCell::new(t),
            main_thread_name: thread::current().name().map(|v| v.to_owned()),
        }
    }

    // TODO: return some sort of ref guard here
    pub fn get(&self) -> &mut T {
        if thread::current().name().map(|v| v.to_owned()) != self.main_thread_name {
            panic!("Can access value only from main thread");
        }

        unsafe { &mut *self.data.get() }
    }

    pub fn safe_call<F, I>(mutex: Arc<MainLoopMutex<I>>, cb: F)
        where I: 'static,
              F: Fn(&MainLoopMutex<I>) + 'static
    {
        gtk::idle_add(move || {
            cb(&*mutex);
            gtk::Continue(false)
        });
    }
}

pub struct Nvim {
    nvim: Neovim,
}

pub trait RedrawEvents {
}

impl Nvim {
    pub fn start(mut ui: Ui) -> Result<Nvim> {
        // let mut session = try!(Session::new_tcp("127.0.0.1:6666"));
        let mut session = if cfg!(target_os = "windows") {
            Session::new_child_path("E:\\Neovim\\bin\\nvim.exe").unwrap()
        } else {
            Session::new_child().unwrap()
        };
        let mut nvim = Neovim::new(session);

        nvim.session.start_event_loop_cb(move |m, p| Nvim::cb(&mut ui, m, p));
        // fix neovim --embed bug to start embed mode
        nvim.input("i").unwrap();
        try!(nvim.ui_attach(80, 24, true).map_err(|e| Error::new(ErrorKind::Other, e)));

        Ok(Nvim { nvim: nvim })
    }

    fn cb(ui: &mut Ui, method: &str, params: Vec<Value>) {
        if method == "redraw" {
            for ev in params {
                if let Value::Array(ev_args) = ev {
                    if let Value::String(ref ev_name) = ev_args[0] {
                        println!("Event {}", ev_name);
                    } else {
                        println!("Unsupported event {:?}", ev_args);
                    }
                } else {
                    println!("Unsupported event type {:?}", ev);
                }
            }
        } else {
            println!("Notification {}", method);
        }
    }
}
