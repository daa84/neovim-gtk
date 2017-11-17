use std::result;
use std::collections::HashMap;

use neovim_lib::{Value, UiOption};
use neovim_lib::neovim_api::Tabpage;

use shell;

use value::ValueMapExt;
use rmpv;

use super::repaint_mode::RepaintMode;
use super::mode_info::ModeInfo;

pub trait RedrawEvents {
    fn on_cursor_goto(&mut self, row: u64, col: u64) -> RepaintMode;

    fn on_put(&mut self, text: String) -> RepaintMode;

    fn on_clear(&mut self) -> RepaintMode;

    fn on_resize(&mut self, columns: u64, rows: u64) -> RepaintMode;

    fn on_redraw(&mut self, mode: &RepaintMode);

    fn on_highlight_set(&mut self, attrs: HashMap<String, Value>) -> RepaintMode;

    fn on_eol_clear(&mut self) -> RepaintMode;

    fn on_set_scroll_region(&mut self, top: u64, bot: u64, left: u64, right: u64) -> RepaintMode;

    fn on_scroll(&mut self, count: i64) -> RepaintMode;

    fn on_update_bg(&mut self, bg: i64) -> RepaintMode;

    fn on_update_fg(&mut self, fg: i64) -> RepaintMode;

    fn on_update_sp(&mut self, sp: i64) -> RepaintMode;

    fn on_mode_change(&mut self, mode: &str, idx: u64) -> RepaintMode;

    fn on_mouse(&mut self, on: bool) -> RepaintMode;

    fn on_busy(&mut self, busy: bool) -> RepaintMode;

    fn popupmenu_show(
        &mut self,
        menu: &[Vec<&str>],
        selected: i64,
        row: u64,
        col: u64,
    ) -> RepaintMode;

    fn popupmenu_hide(&mut self) -> RepaintMode;

    fn popupmenu_select(&mut self, selected: i64) -> RepaintMode;

    fn tabline_update(
        &mut self,
        selected: Tabpage,
        tabs: Vec<(Tabpage, Option<String>)>,
    ) -> RepaintMode;

    fn mode_info_set(
        &mut self,
        cursor_style_enabled: bool,
        mode_info: Vec<ModeInfo>,
    ) -> RepaintMode;

    fn cmdline_show(
        &mut self,
        content: Vec<(HashMap<String, Value>, String)>,
        pos: u64,
        firstc: String,
        prompt: String,
        indent: u64,
        level: u64,
    ) -> RepaintMode;
}

pub trait GuiApi {
    fn set_font(&mut self, font_desc: &str);
}

macro_rules! try_str {
    ($exp:expr) => ($exp.as_str().ok_or_else(|| "Can't convert argument to string".to_owned())?)
}

macro_rules! try_int {
    ($expr:expr) => ($expr.as_i64().ok_or_else(|| "Can't convert argument to int".to_owned())?)
}

macro_rules! try_uint {
    ($exp:expr) => ($exp.as_u64().ok_or_else(|| "Can't convert argument to u64".to_owned())?)
}

macro_rules! try_bool {
    ($exp:expr) => ($exp.as_bool().ok_or_else(|| "Can't convert argument to bool".to_owned())?)
}

macro_rules! map_array {
    ($arg:expr, $err:expr, |$item:ident| $exp:expr) => (
        $arg.as_array()
            .ok_or_else(|| $err)
            .and_then(|items| items.iter().map(|$item| {
                $exp
            }).collect::<Result<Vec<_>, _>>())
    );
    ($arg:expr, $err:expr, |$item:ident| {$exp:expr}) => (
        $arg.as_array()
            .ok_or_else(|| $err)
            .and_then(|items| items.iter().map(|$item| {
                $exp
            }).collect::<Result<Vec<_>, _>>())
    );
}

macro_rules! try_arg {
    ($value:expr, bool) => (try_bool!($value));
    ($value:expr, uint) => (try_uint!($value));
    ($value:expr, str) => (
        match $value {
            Value::String(s) => {
                if let Some(s) = s.into_str() {
                    Ok(s)
                } else {
                    Err("Can't convert to utf8 string".to_owned())
                }
            }
            _ => Err("Can't convert to string".to_owned()),
        }?);
    ($value:expr, ext) => (rmpv::ext::from_value($value).map_err(|e| e.to_string())?);
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

pub fn call_gui_event(
    ui: &mut shell::State,
    method: &str,
    args: &Vec<Value>,
) -> result::Result<(), String> {
    match method {
        "Font" => ui.set_font(try_str!(args[0])),
        "Option" => {
            match try_str!(args[0]) {
                "Popupmenu" => {
                    ui.nvim()
                        .ok_or_else(|| "Nvim not initialized".to_owned())
                        .and_then(|mut nvim| {
                            nvim.set_option(UiOption::ExtPopupmenu(try_uint!(args[1]) == 1))
                                .map_err(|e| e.to_string())
                        })?
                }
                "Tabline" => {
                    ui.nvim()
                        .ok_or_else(|| "Nvim not initialized".to_owned())
                        .and_then(|mut nvim| {
                            nvim.set_option(UiOption::ExtTabline(try_uint!(args[1]) == 1))
                                .map_err(|e| e.to_string())
                        })?
                }
                opt => error!("Unknown option {}", opt),
            }
        }
        _ => return Err(format!("Unsupported event {}({:?})", method, args)),
    }
    Ok(())
}

pub fn call(
    ui: &mut shell::State,
    method: &str,
    args: Vec<Value>,
) -> result::Result<RepaintMode, String> {
    let repaint_mode = match method {
        "cursor_goto" => call!(ui->on_cursor_goto(args: uint, uint)),
        "put" => call!(ui->on_put(args: str)),
        "clear" => ui.on_clear(),
        "resize" => call!(ui->on_resize(args: uint, uint)),
        "highlight_set" => {
            call!(ui->on_highlight_set(args: ext));
            RepaintMode::Nothing
        }
        "eol_clear" => ui.on_eol_clear(),
        "set_scroll_region" => {
            call!(ui->on_set_scroll_region(args: uint, uint, uint, uint));
            RepaintMode::Nothing
        }
        "scroll" => ui.on_scroll(try_int!(args[0])),
        "update_bg" => ui.on_update_bg(try_int!(args[0])),
        "update_fg" => ui.on_update_fg(try_int!(args[0])),
        "update_sp" => ui.on_update_sp(try_int!(args[0])),
        "mode_change" => ui.on_mode_change(try_str!(args[0]), try_uint!(args[1])),
        "mouse_on" => ui.on_mouse(true),
        "mouse_off" => ui.on_mouse(false),
        "busy_start" => ui.on_busy(true),
        "busy_stop" => ui.on_busy(false),
        "popupmenu_show" => {
            let menu_items = map_array!(args[0], "Error get menu list array", |item| {
                map_array!(item, "Error get menu item array", |col| {
                    col.as_str().ok_or("Error get menu column")
                })
            })?;

            ui.popupmenu_show(
                &menu_items,
                try_int!(args[1]),
                try_uint!(args[2]),
                try_uint!(args[3]),
            )
        }
        "popupmenu_hide" => ui.popupmenu_hide(),
        "popupmenu_select" => ui.popupmenu_select(try_int!(args[0])),
        "tabline_update" => {
            let tabs_out = map_array!(args[1], "Error get tabline list".to_owned(), |tab| {
                tab.as_map()
                    .ok_or_else(|| "Error get map for tab".to_owned())
                    .and_then(|tab_map| tab_map.to_attrs_map())
                    .map(|tab_attrs| {
                        let name_attr = tab_attrs.get("name").and_then(
                            |n| n.as_str().map(|s| s.to_owned()),
                        );
                        let tab_attr = tab_attrs
                            .get("tab")
                            .map(|&tab_id| Tabpage::new(tab_id.clone()))
                            .unwrap();

                        (tab_attr, name_attr)
                    })
            })?;
            ui.tabline_update(Tabpage::new(args[0].clone()), tabs_out)
        }
        "mode_info_set" => {
            let mode_info = map_array!(
                args[1],
                "Error get array key value for mode_info".to_owned(),
                |mi| {
                    mi.as_map()
                        .ok_or_else(|| "Erro get map for mode_info".to_owned())
                        .and_then(|mi_map| ModeInfo::new(mi_map))
                }
            )?;
            ui.mode_info_set(try_bool!(args[0]), mode_info)
        }
        "cmdline_show" => call!(ui->cmdline_show(args: ext, uint, str, str, uint, uint)),
        _ => {
            println!("Event {}({:?})", method, args);
            RepaintMode::Nothing
        }
    };

    Ok(repaint_mode)
}
