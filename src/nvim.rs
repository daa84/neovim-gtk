use neovim_lib::{Neovim, NeovimApi, Session};
use std::io::{Result, Error, ErrorKind};
use std::result;
use std::collections::HashMap;
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

    fn on_redraw(&self);

    fn on_highlight_set(&mut self, attrs: &HashMap<String, Value>);

    fn on_eol_clear(&mut self);

    fn on_set_scroll_region(&mut self, top: u64, bot: u64, left: u64, right: u64);

    fn on_scroll(&mut self, count: i64);

    fn on_update_bg(&mut self, bg: i64);

    fn on_update_fg(&mut self, fg: i64);
}

macro_rules! try_str {
    ($exp:expr) => (match $exp {
        Value::String(ref val) => val,
        _ => return Err("Can't convert argument to string".to_owned())
    })
}

macro_rules! try_int {
    ($expr:expr) => (match $expr {
        Value::Integer(Integer::U64(val)) => val as i64,
        Value::Integer(Integer::I64(val)) => val,
        _ =>  return Err("Can't convert argument to int".to_owned())
    })
}

macro_rules! try_uint {
    ($exp:expr) => (match $exp {
        Value::Integer(Integer::U64(val)) => val,
        _ => return Err("Can't convert argument to u64".to_owned())
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
    ui.model = UiModel::new(24, 80);

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

        safe_call(move |ui| {
            ui.on_redraw();
            Ok(())
        });
    } else {
        println!("Notification {}", method);
    }
}

fn call(method: &str, args: Vec<Value>) {
    match method {
        "cursor_goto" => {
            safe_call(move |ui| {
                ui.on_cursor_goto(try_uint!(args[0]), try_uint!(args[1]));
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
                ui.on_resize(try_uint!(args[0]), try_uint!(args[1]));
                Ok(())
            });
        }
        "highlight_set" => {
            safe_call(move |ui| {
                if let Value::Map(ref attrs) = args[0] {
                    let attrs_map: HashMap<String, Value> = attrs.iter()
                                                                 .map(|v| {
                                                                     match v {
                                                                         &(Value::String(ref key),
                                                                           ref value) => {
                                                                             (key.clone(),
                                                                              value.clone())
                                                                         }
                                                                         _ => {
                                                                             panic!("attribute \
                                                                                     key must be \
                                                                                     string")
                                                                         }
                                                                     }
                                                                 })
                                                                 .collect();
                    ui.on_highlight_set(&attrs_map);
                } else {
                    panic!("Supports only map value as argument");
                }
                Ok(())
            });
        }
        "eol_clear" => {
            safe_call(move |ui| {
                ui.on_eol_clear();
                Ok(())
            })
        }
        "set_scroll_region" => {
            safe_call(move |ui| { 
                ui.on_set_scroll_region(try_uint!(args[0]), try_uint!(args[1]), try_uint!(args[2]), try_uint!(args[3]));
                Ok(())
            });
        }
        "scroll" => {
            safe_call(move |ui| { 
                ui.on_scroll(try_int!(args[0]));
                Ok(())
            });
        }
        "update_bg" => {
            safe_call(move |ui| { 
                ui.on_update_bg(try_int!(args[0]));
                Ok(())
            });
        }
        "update_fg" => {
            safe_call(move |ui| { 
                ui.on_update_fg(try_int!(args[0]));
                Ok(())
            });
        }
        _ => println!("Event {}({:?})", method, args),
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
