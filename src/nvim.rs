use neovim_lib::{Neovim, NeovimApi, Session};
use std::io::{Result, Error, ErrorKind};
use rmp::Value;

pub struct Nvim {
    nvim: Neovim,
}

pub trait RedrawEvents {
}

impl Nvim {
    pub fn start<F: RedrawEvents + Send + 'static> (redraw_cb: F) -> Result<Nvim> {
        // let mut session = try!(Session::new_tcp("127.0.0.1:6666"));
        let mut session = try!(Session::new_child());

        session.start_event_loop_cb(move |m, p| Nvim::cb(&redraw_cb, m, p));

        let mut nvim = Neovim::new(session);
        // fix neovim --embed bug to start embed mode
        nvim.input("i").unwrap();
        try!(nvim.ui_attach(80, 24, true).map_err(|e| Error::new(ErrorKind::Other, e)));

        Ok(Nvim { nvim: nvim })
    }

    fn cb<F: RedrawEvents>(readraw_cb: &F, method: &str, params: Vec<Value>) {
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
