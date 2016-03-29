use neovim_lib::{Neovim, NeovimApi, Session};
use std::io::{Result, Error, ErrorKind};
use std::sync::Arc;
use std::result;
use ui_model::UiModel;
use ui_mutex::UiMutex;
use rmp::Value;
use rmp::value::Integer;
use ui::Ui;
use glib;

pub type SharedUi = Arc<UiMutex<Ui>>;

pub trait RedrawEvents {
    fn on_cursor_goto(&mut self, row: u64, col: u64);

    fn on_put(&mut self, text: &str);

    fn on_clear(&mut self);
}

macro_rules! try_str {
    ($exp:expr) => (match $exp {
        Value::String(ref val) => val,
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
    ui.model = UiModel::new(80, 24);

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
                    let args = match ev_args[1] {
                        Value::Array(ref ar) => ar.clone(),
                        _ => vec![],
                    };
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
            safe_call(ui, move |ui| {
                ui.borrow_mut().on_cursor_goto(try_int!(args[0]), try_int!(args[1]));
                Ok(())
            })
        }
        "put" => {
            safe_call(ui, move |ui| {
                ui.borrow_mut().on_put(try_str!(args[0]));
                Ok(())
            })
        }
        "clear" => {
            safe_call(ui, move |ui| {
                ui.borrow_mut().on_clear();
                Ok(())
            })
        }
        _ => println!("Event {}", method),
    };
}

fn safe_call<F>(ui: &SharedUi, cb: F)
    where F: Fn(&UiMutex<Ui>) -> result::Result<(), String> + 'static + Send
{
    let sent_ui = ui.clone();
    glib::idle_add(move || {
        if let Err(msg) = cb(&*sent_ui) {
            println!("Error call function: {}", msg);
        }
        glib::Continue(false)
    });
}
