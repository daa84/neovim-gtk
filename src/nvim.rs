use neovim_lib::{Neovim, NeovimApi, Session};
use std::io::{Result, Error, ErrorKind};
use std::result;
use ui_model::UiModel;
use rmp::Value;
use rmp::value::Integer;
use ui;
use ui::Ui;
use glib;

pub trait RedrawEvents {
    fn on_cursor_goto(&mut self, row: u64, col: u64);

    fn on_put(&mut self, text: &str);

    fn on_clear(&mut self);

    fn on_resize(&mut self, columns: u64, rows: u64);
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

pub fn initialize(ui: &mut Ui) -> Result<()> {
    // let mut session = try!(Session::new_tcp("127.0.0.1:6666"));
    let session = if cfg!(target_os = "windows") {
        Session::new_child_path("E:\\Neovim\\bin\\nvim.exe").unwrap()
    } else {
        Session::new_child().unwrap()
    };
    let nvim = Neovim::new(session);
    ui.set_nvim(nvim);
    ui.model = UiModel::new(80, 24);

    let mut nvim = ui.nvim();

    nvim.session.start_event_loop_cb(move |m, p| nvim_cb(m, p));
    // fix neovim --embed bug to start embed mode
    nvim.input("i").unwrap();
    try!(nvim.ui_attach(80, 24, true).map_err(|e| Error::new(ErrorKind::Other, e)));

    Ok(())
}

fn nvim_cb(method: &str, params: Vec<Value>) {
    if method == "redraw" {
        for ev in params {
            if let Value::Array(ev_args) = ev {
                if let Value::String(ref ev_name) = ev_args[0] {
                    for ref local_args in ev_args.iter().skip(1) {
                        let args = match *local_args {
                            &Value::Array(ref ar) => ar.clone(),
                            _ => vec![],
                        };
                        call(ev_name, args);
                    }
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

fn call(method: &str, args: Vec<Value>) {
    match method {
        "cursor_goto" => {
            safe_call(move |ui| {
                ui.on_cursor_goto(try_int!(args[0]), try_int!(args[1]));
                Ok(())
            })
        }
        "put" => {
            safe_call(move |ui| {
                ui.on_put(try_str!(args[0]));
                Ok(())
            })
        }
        "clear" => {
            safe_call(move |ui| {
                ui.on_clear();
                Ok(())
            })
        }
        "resize" => {
            safe_call(move |ui| {
                ui.on_resize(try_int!(args[0]), try_int!(args[1]));
                Ok(())
            });
        }
        _ => println!("Event {}", method),
    };
}

fn safe_call<F>(cb: F)
    where F: Fn(&mut Ui) -> result::Result<(), String> + 'static + Send
{
    glib::idle_add(move || {
        ui::UI.with(|ui_cell| {
            if let Err(msg) = cb(&mut *ui_cell.borrow_mut()) {
                println!("Error call function: {}", msg);
            }
        });
        glib::Continue(false)
    });
}
