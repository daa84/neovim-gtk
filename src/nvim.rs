use neovim_lib::{Neovim, NeovimApi, Session};
use std::io::{Result, Error, ErrorKind};
use std::sync::Arc;
use std::result;
use ui_mutex::UiMutex;
use rmp::Value;
use rmp::value::Integer;
use ui::Ui;
use gtk;

pub type SharedUi = Arc<UiMutex<Ui>>;

pub trait RedrawEvents {
    fn on_cursor_goto(&mut self, row: u64, col: u64);
}

macro_rules! try_str {
    ($exp:expr) => (match $exp {
        Value::String(ref val) => val.to_owned(),
        _ => return Err("Can't convert argument to string".to_owned())
    })
}

macro_rules! try_int {
    ($exp:expr) => (match $exp {
        Value::Integer(Integer::U64(val)) => val,
        _ => return Err("Can't convert argument to int".to_owned())
    })
}

pub fn initialize(mut ui: Ui) -> Result<SharedUi> {
    // let mut session = try!(Session::new_tcp("127.0.0.1:6666"));
    let session = if cfg!(target_os = "windows") {
        Session::new_child_path("E:\\Neovim\\bin\\nvim.exe").unwrap()
    } else {
        Session::new_child().unwrap()
    };
    let nvim = Neovim::new(session);
    ui.set_nvim(nvim);

    let sh_ui = Arc::new(UiMutex::new(ui));


    {
        let mut ui = (*sh_ui).borrow_mut();
        let mut nvim = ui.nvim();

        let moved_sh_ui = sh_ui.clone();
        nvim.session.start_event_loop_cb(move |m, p| nvim_cb(&moved_sh_ui, m, p));
        // fix neovim --embed bug to start embed mode
        nvim.input("i").unwrap();
        try!(nvim.ui_attach(80, 24, true).map_err(|e| Error::new(ErrorKind::Other, e)));
    }

    Ok(sh_ui)
}

fn nvim_cb(ui: &SharedUi, method: &str, params: Vec<Value>) {
    if method == "redraw" {
        for ev in params {
            if let Value::Array(ev_args) = ev {
                if let Value::String(ref ev_name) = ev_args[0] {
                    let mut args = vec![];
                    args.extend_from_slice(&ev_args[1..]);
                    call(ui, ev_name, args);
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

fn call(ui: &SharedUi, method: &str, args: Vec<Value>) {
    match method {
        "cursor_goto" => {
            safe_call(ui.clone(), move |ui| {
                ui.borrow_mut().on_cursor_goto(try_int!(args[0]), try_int!(args[1]));
                Ok(())
            })
        }
        _ => println!("Event {}", method),
    };
}

fn safe_call<F>(mutex: SharedUi, cb: F)
    where F: Fn(&UiMutex<Ui>) -> result::Result<(), String> + 'static
{
    gtk::idle_add(move || {
        if let Err(msg) = cb(&*mutex) {
            println!("Error call function: {}", msg);
        }
        gtk::Continue(false)
    });
}
