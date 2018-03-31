use std::result;
use std::sync::{mpsc, Arc};

use neovim_lib::{Handler, Value};

use ui::UiMutex;
use shell;
use glib;

use super::repaint_mode::RepaintMode;
use super::redraw_handler;

pub struct NvimHandler {
    shell: Arc<UiMutex<shell::State>>,

    delayed_redraw_event_id: Arc<UiMutex<Option<glib::SourceId>>>,
}

impl NvimHandler {
    pub fn new(shell: Arc<UiMutex<shell::State>>) -> NvimHandler {
        NvimHandler {
            shell,
            delayed_redraw_event_id: Arc::new(UiMutex::new(None)),
        }
    }

    pub fn schedule_redraw_event(&self, event: Value) {
        let shell = self.shell.clone();
        let delayed_redraw_event_id = self.delayed_redraw_event_id.clone();

        glib::idle_add(move || {
            let id = Some(glib::timeout_add(
                250,
                clone!(shell, event, delayed_redraw_event_id => move || {
                delayed_redraw_event_id.replace(None);

                if let Err(msg) = call_redraw_handler(vec![event.clone()], &shell) {
                    error!("Error call function: {}", msg);
                }

                glib::Continue(false)
            }),
            ));

            delayed_redraw_event_id.replace(id);

            glib::Continue(false)
        });
    }

    pub fn remove_scheduled_redraw_event(&self) {
        let delayed_redraw_event_id = self.delayed_redraw_event_id.clone();
        glib::idle_add(move || {
            let id = delayed_redraw_event_id.replace(None);
            if let Some(ev_id) = id {
                glib::source_remove(ev_id);
            }

            glib::Continue(false)
        });
    }

    fn nvim_cb(&self, method: &str, mut params: Vec<Value>) {
        match method {
            "redraw" => {
                redraw_handler::remove_or_delay_uneeded_events(self, &mut params);

                self.safe_call(move |ui| call_redraw_handler(params, ui));
            }
            "Gui" => {
                if !params.is_empty() {
                    let mut params_iter = params.into_iter();
                    if let Some(ev_name) = params_iter.next() {
                        if let Value::String(ev_name) = ev_name {
                            let args = params_iter.collect();
                            self.safe_call(move |ui| {
                                let ui = &mut ui.borrow_mut();
                                redraw_handler::call_gui_event(
                                    ui,
                                    ev_name
                                        .as_str()
                                        .ok_or_else(|| "Event name does not exists")?,
                                    args,
                                )?;
                                ui.on_redraw(&RepaintMode::All);
                                Ok(())
                            });
                        } else {
                            error!("Unsupported event");
                        }
                    } else {
                        error!("Event name does not exists");
                    }
                } else {
                    error!("Unsupported event {:?}", params);
                }
            }
            "subscription" => {
                self.safe_call(move |ui| {
                    let ui = &ui.borrow();
                    ui.notify(params)
                });
            }
            _ => {
                error!("Notification {}({:?})", method, params);
            }
        }
    }

    fn nvim_cb_req(&self, method: &str, params: Vec<Value>) -> result::Result<Value, Value> {
        match method {
            "Gui" => {
                if !params.is_empty() {
                    let mut params_iter = params.into_iter();
                    if let Some(req_name) = params_iter.next() {
                        if let Value::String(req_name) = req_name {
                            let args = params_iter.collect();
                            let (sender, receiver) = mpsc::channel();
                            self.safe_call(move |ui| {
                                sender
                                    .send(redraw_handler::call_gui_request(
                                        &ui.clone(),
                                        req_name
                                            .as_str()
                                            .ok_or_else(|| "Event name does not exists")?,
                                        &args,
                                    ))
                                    .unwrap();
                                {
                                    let ui = &mut ui.borrow_mut();
                                    ui.on_redraw(&RepaintMode::All);
                                }
                                Ok(())
                            });
                            Ok(receiver.recv().unwrap()?)
                        } else {
                            error!("Unsupported request");
                            Err(Value::Nil)
                        }
                    } else {
                        error!("Request name does not exist");
                        Err(Value::Nil)
                    }
                } else {
                    error!("Unsupported request {:?}", params);
                    Err(Value::Nil)
                }
            }
            _ => {
                error!("Request {}({:?})", method, params);
                Err(Value::Nil)
            }
        }
    }

    fn safe_call<F>(&self, cb: F)
    where
        F: FnOnce(&Arc<UiMutex<shell::State>>) -> result::Result<(), String> + 'static + Send,
    {
        safe_call(self.shell.clone(), cb);
    }
}

fn call_redraw_handler(
    params: Vec<Value>,
    ui: &Arc<UiMutex<shell::State>>,
) -> result::Result<(), String> {
    let ui = &mut ui.borrow_mut();
    let mut repaint_mode = RepaintMode::Nothing;

    for ev in params {
        if let Value::Array(ev_args) = ev {
            let mut args_iter = ev_args.into_iter();
            let ev_name = args_iter.next();
            if let Some(ev_name) = ev_name {
                if let Some(ev_name) = ev_name.as_str() {
                    for local_args in args_iter {
                        let args = match local_args {
                            Value::Array(ar) => ar,
                            _ => vec![],
                        };
                        let call_reapint_mode = redraw_handler::call(ui, &ev_name, args)?;
                        repaint_mode = repaint_mode.join(call_reapint_mode);
                    }
                } else {
                    error!("Unsupported event");
                }
            } else {
                error!("Event name does not exists");
            }
        } else {
            error!("Unsupported event type {:?}", ev);
        }
    }

    ui.on_redraw(&repaint_mode);
    Ok(())
}

fn safe_call<F>(shell: Arc<UiMutex<shell::State>>, cb: F)
where
    F: FnOnce(&Arc<UiMutex<shell::State>>) -> result::Result<(), String> + 'static + Send,
{
    let mut cb = Some(cb);
    glib::idle_add(move || {
        if let Err(msg) = cb.take().unwrap()(&shell) {
            error!("Error call function: {}", msg);
        }
        glib::Continue(false)
    });
}

impl Handler for NvimHandler {
    fn handle_notify(&mut self, name: &str, args: Vec<Value>) {
        self.nvim_cb(name, args);
    }

    fn handle_request(&mut self, name: &str, args: Vec<Value>) -> result::Result<Value, Value> {
        self.nvim_cb_req(name, args)
    }
}
