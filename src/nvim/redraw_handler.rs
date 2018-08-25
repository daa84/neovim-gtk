use std::num::ParseFloatError;
use std::result;
use std::sync::Arc;

use neovim_lib::neovim_api::Tabpage;
use neovim_lib::{UiOption, Value};

use gtk::ClipboardExt;
use shell;
use ui::UiMutex;

use rmpv;
use value::ValueMapExt;

use super::handler::NvimHandler;
use super::repaint_mode::RepaintMode;

macro_rules! try_str {
    ($exp:expr) => {
        $exp.as_str()
            .ok_or_else(|| "Can't convert argument to string".to_owned())?
    };
}

macro_rules! try_int {
    ($expr:expr) => {
        $expr
            .as_i64()
            .ok_or_else(|| "Can't convert argument to int".to_owned())?
    };
}

macro_rules! try_uint {
    ($exp:expr) => {
        $exp.as_u64()
            .ok_or_else(|| "Can't convert argument to u64".to_owned())?
    };
}

macro_rules! try_bool {
    ($exp:expr) => {
        $exp.as_bool()
            .ok_or_else(|| "Can't convert argument to bool".to_owned())?
    };
}

macro_rules! map_array {
    ($arg:expr, $err:expr, | $item:ident | $exp:expr) => {
        $arg.as_array().ok_or_else(|| $err).and_then(|items| {
            items
                .iter()
                .map(|$item| $exp)
                .collect::<Result<Vec<_>, _>>()
        })
    };
    ($arg:expr, $err:expr, | $item:ident |  { $exp:expr }) => {
        $arg.as_array().ok_or_else(|| $err).and_then(|items| {
            items
                .iter()
                .map(|$item| $exp)
                .collect::<Result<Vec<_>, _>>()
        })
    };
}

macro_rules! try_arg {
    ($value:expr,bool) => {
        try_bool!($value)
    };
    ($value:expr,uint) => {
        try_uint!($value)
    };
    ($value:expr,int) => {
        try_int!($value)
    };
    ($value:expr,float) => {
        try_float!($value)
    };
    ($value:expr,str) => {
        match $value {
            Value::String(s) => {
                if let Some(s) = s.into_str() {
                    Ok(s)
                } else {
                    Err("Can't convert to utf8 string".to_owned())
                }
            }
            _ => Err("Can't convert to string".to_owned()),
        }?
    };
    ($value:expr,ext) => {
        rmpv::ext::from_value($value).map_err(|e| e.to_string())?
    };
}

macro_rules! call {
    ($s:ident -> $c:ident ($args:ident : $($arg_type:ident),+ )) => (
        {
            let mut iter = $args.into_iter();
            $s.$c($(
                try_arg!(iter.next()
                             .ok_or_else(|| format!("No such argument for {}", stringify!($c)))?,
                         $arg_type
                        )
            ),+ )
        }
    )
}

pub enum NvimCommand {
    ToggleSidebar,
    Transparency(f64, f64),
}

pub fn call_gui_event(
    ui: &mut shell::State,
    method: &str,
    args: Vec<Value>,
) -> result::Result<(), String> {
    match method {
        "Font" => call!(ui->set_font(args: str)),
        "FontFeatures" => call!(ui->set_font_features(args: str)),
        "Linespace" => call!(ui->set_line_space(args: str)),
        "Clipboard" => match try_str!(args[0]) {
            "Set" => match try_str!(args[1]) {
                "*" => ui.clipboard_primary_set(try_str!(args[2])),
                _ => ui.clipboard_clipboard_set(try_str!(args[2])),
            },
            opt => error!("Unknown option {}", opt),
        },
        "Option" => match try_str!(args[0]) {
            "Popupmenu" => ui.nvim()
                .ok_or_else(|| "Nvim not initialized".to_owned())
                .and_then(|mut nvim| {
                    nvim.set_option(UiOption::ExtPopupmenu(try_uint!(args[1]) == 1))
                        .map_err(|e| e.to_string())
                })?,
            "Tabline" => ui.nvim()
                .ok_or_else(|| "Nvim not initialized".to_owned())
                .and_then(|mut nvim| {
                    nvim.set_option(UiOption::ExtTabline(try_uint!(args[1]) == 1))
                        .map_err(|e| e.to_string())
                })?,
            "Cmdline" => ui.nvim()
                .ok_or_else(|| "Nvim not initialized".to_owned())
                .and_then(|mut nvim| {
                    nvim.set_option(UiOption::ExtCmdline(try_uint!(args[1]) == 1))
                        .map_err(|e| e.to_string())?;
                    nvim.set_option(UiOption::ExtWildmenu(try_uint!(args[1]) == 1))
                        .map_err(|e| e.to_string())
                })?,
            opt => error!("Unknown option {}", opt),
        },
        "Command" => {
            match try_str!(args[0]) {
                "ToggleSidebar" => ui.on_command(NvimCommand::ToggleSidebar),
                "Transparency" => ui.on_command(NvimCommand::Transparency(
                    try_str!(args.get(1).cloned().unwrap_or("1.0".into()))
                        .parse()
                        .map_err(|e: ParseFloatError| e.to_string())?,
                    try_str!(args.get(2).cloned().unwrap_or("1.0".into()))
                        .parse()
                        .map_err(|e: ParseFloatError| e.to_string())?,
                )),
                _ => error!("Unknown command"),
            };
        }
        _ => return Err(format!("Unsupported event {}({:?})", method, args)),
    }
    Ok(())
}

pub fn call_gui_request(
    ui: &Arc<UiMutex<shell::State>>,
    method: &str,
    args: &Vec<Value>,
) -> result::Result<Value, Value> {
    match method {
        "Clipboard" => {
            match try_str!(args[0]) {
                "Get" => {
                    // NOTE: wait_for_text waits on the main loop. We can't have the ui borrowed
                    // while it runs, otherwise ui callbacks will get called and try to borrow
                    // mutably twice!
                    let clipboard = {
                        let ui = &mut ui.borrow_mut();
                        match try_str!(args[1]) {
                            "*" => ui.clipboard_primary.clone(),
                            _ => ui.clipboard_clipboard.clone(),
                        }
                    };
                    let t = clipboard.wait_for_text().unwrap_or_else(|| String::new());
                    Ok(Value::Array(
                        t.split("\n").map(|s| s.into()).collect::<Vec<Value>>(),
                    ))
                }
                opt => {
                    error!("Unknown option {}", opt);
                    Err(Value::Nil)
                }
            }
        }
        _ => Err(Value::String(
            format!("Unsupported request {}({:?})", method, args).into(),
        )),
    }
}

pub fn call(
    ui: &mut shell::State,
    method: &str,
    args: Vec<Value>,
) -> result::Result<RepaintMode, String> {
    let repaint_mode = match method {
        "grid_line" => call!(ui->grid_line(args: uint, uint, uint, ext)),
        "grid_clear" => call!(ui->grid_clear(args: uint)),
        "grid_destroy" => call!(ui->grid_destroy(args: uint)),
        "grid_cursor_goto" => call!(ui->grid_cursor_goto(args: uint, uint, uint)),
        "grid_scroll" => call!(ui->grid_scroll(args: uint, uint, uint, uint, uint, int, int)),
        "grid_resize" => call!(ui->grid_resize(args: uint, uint, uint)),
        "default_colors_set" => call!(ui->default_colors_set(args: uint, uint, uint)),
        //"cursor_goto" => call!(ui->on_cursor_goto(args: uint, uint)),
        //"put" => call!(ui->on_put(args: str)),
        //"clear" => ui.on_clear(),
        //"resize" => call!(ui->on_resize(args: uint, uint)),
        //"highlight_set" => {
        //    call!(ui->on_highlight_set(args: ext));
        //    RepaintMode::Nothing
        //}
        //"eol_clear" => ui.on_eol_clear(),
        //"set_scroll_region" => {
        //    call!(ui->on_set_scroll_region(args: uint, uint, uint, uint));
        //    RepaintMode::Nothing
        //}
        //"scroll" => call!(ui->on_scroll(args: int)),
        //"update_bg" => call!(ui->on_update_bg(args: int)),
        //"update_fg" => call!(ui->on_update_fg(args: int)),
        //"update_sp" => call!(ui->on_update_sp(args: int)),
        "mode_change" => call!(ui->on_mode_change(args: str, uint)),
        "mouse_on" => ui.on_mouse(true),
        "mouse_off" => ui.on_mouse(false),
        "busy_start" => ui.on_busy(true),
        "busy_stop" => ui.on_busy(false),
        "popupmenu_show" => {
            let menu_items = map_array!(args[0], "Error get menu list array", |item| map_array!(
                item,
                "Error get menu item array",
                |col| col.as_str().ok_or("Error get menu column")
            ))?;

            ui.popupmenu_show(
                &CompleteItem::map(&menu_items),
                try_int!(args[1]),
                try_uint!(args[2]),
                try_uint!(args[3]),
            )
        }
        "popupmenu_hide" => ui.popupmenu_hide(),
        "popupmenu_select" => call!(ui->popupmenu_select(args: int)),
        "tabline_update" => {
            let tabs_out = map_array!(
                args[1],
                "Error get tabline list".to_owned(),
                |tab| tab.as_map()
                    .ok_or_else(|| "Error get map for tab".to_owned())
                    .and_then(|tab_map| tab_map.to_attrs_map())
                    .map(|tab_attrs| {
                        let name_attr = tab_attrs
                            .get("name")
                            .and_then(|n| n.as_str().map(|s| s.to_owned()));
                        let tab_attr = tab_attrs
                            .get("tab")
                            .map(|&tab_id| Tabpage::new(tab_id.clone()))
                            .unwrap();

                        (tab_attr, name_attr)
                    })
            )?;
            ui.tabline_update(Tabpage::new(args[0].clone()), tabs_out)
        }
        "mode_info_set" => call!(ui->mode_info_set(args: bool, ext)),
        "cmdline_show" => call!(ui->cmdline_show(args: ext, uint, str, str, uint, uint)),
        "cmdline_block_show" => call!(ui->cmdline_block_show(args: ext)),
        "cmdline_block_append" => call!(ui->cmdline_block_append(args: ext)),
        "cmdline_hide" => call!(ui->cmdline_hide(args: uint)),
        "cmdline_block_hide" => ui.cmdline_block_hide(),
        "cmdline_pos" => call!(ui->cmdline_pos(args: uint, uint)),
        "cmdline_special_char" => call!(ui->cmdline_special_char(args: str, bool, uint)),
        "wildmenu_show" => call!(ui->wildmenu_show(args: ext)),
        "wildmenu_hide" => ui.wildmenu_hide(),
        "wildmenu_select" => call!(ui->wildmenu_select(args: int)),
        _ => {
            warn!("Event {}({:?})", method, args);
            RepaintMode::Nothing
        }
    };

    Ok(repaint_mode)
}

// Here two cases processed:
//
// 1. menu content update call popupmenu_hide followed by popupmenu_show in same batch
// this generates unneeded hide event
// so in case we get both events, just romove one
//
// 2. postpone hide event when "show" event come bit later
// but in new event batch
pub fn remove_or_delay_uneeded_events(handler: &NvimHandler, params: &mut Vec<Value>) {
    let mut show_popup_finded = false;
    let mut to_remove = Vec::new();
    let mut delayed_hide_event = None;

    for (idx, val) in params.iter().enumerate().rev() {
        if let Some(args) = val.as_array() {
            match args[0].as_str() {
                Some("popupmenu_show") => {
                    show_popup_finded = true;
                    handler.remove_scheduled_redraw_event();
                }
                Some("popupmenu_hide") if !show_popup_finded && delayed_hide_event.is_none() => {
                    to_remove.push(idx);
                    delayed_hide_event = Some(idx);
                    handler.remove_scheduled_redraw_event();
                }
                Some("popupmenu_hide") => {
                    to_remove.push(idx);
                }
                _ => (),
            }
        }
    }

    to_remove.iter().for_each(|&idx| {
        let ev = params.remove(idx);
        if let Some(delayed_hide_event_idx) = delayed_hide_event {
            if delayed_hide_event_idx == idx {
                handler.schedule_redraw_event(ev);
            }
        }
    });
}

pub struct CompleteItem<'a> {
    pub word: &'a str,
    pub kind: &'a str,
    pub menu: &'a str,
    pub info: &'a str,
}

impl<'a> CompleteItem<'a> {
    fn map(menu: &'a [Vec<&str>]) -> Vec<Self> {
        menu.iter()
            .map(|menu| CompleteItem {
                word: menu[0],
                kind: menu[1],
                menu: menu[2],
                info: menu[3],
            })
            .collect()
    }
}
