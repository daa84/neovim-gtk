use neovim_lib::{Neovim, NeovimApi, Session};
use std::io::{Result, Error, ErrorKind};
use rmp::Value;
use ui::Ui;

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
