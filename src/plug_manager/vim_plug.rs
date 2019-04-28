use std::rc::Rc;

use neovim_lib::{NeovimApi, NeovimApiAsync};

use crate::nvim::{NeovimClient, ErrorReport, NeovimRef};
use crate::value::ValueMapExt;

pub struct Manager {
    nvim: Option<Rc<NeovimClient>>,
}

impl Manager {
    pub fn new() -> Self {
        Manager { nvim: None }
    }

    pub fn initialize(&mut self, nvim: Rc<NeovimClient>) {
        self.nvim = Some(nvim);
    }

    fn nvim(&self) -> Option<NeovimRef> {
        self.nvim.as_ref().unwrap().nvim()
    }

    pub fn get_plugs(&self) -> Result<Box<[VimPlugInfo]>, String> {
        if let Some(mut nvim) = self.nvim() {
            let g_plugs = nvim.eval("g:plugs").map_err(|e| {
                format!("Can't retrive g:plugs map: {}", e)
            })?;

            let plugs_map = g_plugs
                .as_map()
                .ok_or("Can't retrive g:plugs map".to_owned())?
                .to_attrs_map()?;

            let g_plugs_order = nvim.eval("g:plugs_order").map_err(|e| format!("{}", e))?;

            let order_arr = g_plugs_order.as_array().ok_or(
                "Can't find g:plugs_order array"
                    .to_owned(),
            )?;

            let plugs_info: Vec<VimPlugInfo> = order_arr
                .iter()
                .map(|n| n.as_str())
                .filter_map(|name| if let Some(name) = name {
                    plugs_map
                        .get(name)
                        .and_then(|desc| desc.as_map())
                        .and_then(|desc| desc.to_attrs_map().ok())
                        .and_then(|desc| {
                            let uri = desc.get("uri").and_then(|uri| uri.as_str());
                            if let Some(uri) = uri {
                                Some(VimPlugInfo::new(name.to_owned(), uri.to_owned()))
                            } else {
                                None
                            }
                        })
                } else {
                    None
                })
                .collect();
            Ok(plugs_info.into_boxed_slice())
        } else {
            Err("Nvim not initialized".to_owned())
        }
    }

    pub fn is_loaded(&self) -> bool {
        if let Some(mut nvim) = self.nvim() {
            let loaded_plug = nvim.eval("exists('g:loaded_plug')");
            loaded_plug
                .ok_and_report()
                .and_then(|loaded_plug| loaded_plug.as_i64())
                .map_or(false, |loaded_plug| if loaded_plug > 0 {
                    true
                } else {
                    false
                })
        } else {
            false
        }
    }

    pub fn reload(&self, path: &str) {
        if let Some(mut nvim) = self.nvim() {
            nvim.command_async(&format!("source {}", path))
                .cb(|r| r.report_err())
                .call()
        }
    }
}

#[derive(Debug)]
pub struct VimPlugInfo {
    pub name: String,
    pub uri: String,
}

impl VimPlugInfo {
    pub fn new(name: String, uri: String) -> Self {
        VimPlugInfo { name, uri }
    }
}
