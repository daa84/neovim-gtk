
use gtk::prelude::*;
use gdk;
use gdk::EventKey;
use phf;
use neovim_lib::{Neovim, NeovimApi};

include!(concat!(env!("OUT_DIR"), "/key_map_table.rs"));


pub fn keyval_to_input_string(in_str: &str, in_state: gdk::ModifierType) -> String {
    let mut val = in_str;
    let mut state = in_state;
    let mut input = String::new();

    debug!("keyval -> {}", in_str);

    // CTRL-^ and CTRL-@ don't work in the normal way.
    if state.contains(gdk::ModifierType::CONTROL_MASK) && !state.contains(gdk::ModifierType::SHIFT_MASK) &&
        !state.contains(gdk::ModifierType::MOD1_MASK)
    {
        if val == "6" {
            val = "^";
        } else if val == "2" {
            val = "@";
        }
    }

    let chars: Vec<char> = in_str.chars().collect();

    if chars.len() == 1 {
        let ch = chars[0];

        // Remove SHIFT
        if ch.is_ascii() && !ch.is_alphanumeric() {
            state.remove(gdk::ModifierType::SHIFT_MASK);
        }
    }

    if val == "<" {
        val = "lt";
    }

    if state.contains(gdk::ModifierType::SHIFT_MASK) {
        input.push_str("S-");
    }
    if state.contains(gdk::ModifierType::CONTROL_MASK) {
        input.push_str("C-");
    }
    if state.contains(gdk::ModifierType::MOD1_MASK) {
        input.push_str("A-");
    }

    input.push_str(val);

    if input.chars().count() > 1 {
        format!("<{}>", input)
    } else {
        input
    }
}

pub fn convert_key(ev: &EventKey) -> Option<String> {
    let keyval = ev.get_keyval();
    let state = ev.get_state();
    if let Some(ref keyval_name) = gdk::keyval_name(keyval) {
        if let Some(cnvt) = KEYVAL_MAP.get(keyval_name as &str).cloned() {
            return Some(keyval_to_input_string(cnvt, state));
        }
    }

    if let Some(ch) = gdk::keyval_to_unicode(keyval) {
        Some(keyval_to_input_string(&ch.to_string(), state))
    } else {
        None
    }
}

pub fn im_input(nvim: &mut Neovim, input: &str) {
    debug!("nvim_input -> {}", input);

    let input: String = input
        .chars()
        .map(|ch| {
            keyval_to_input_string(&ch.to_string(), gdk::ModifierType::empty())
        })
        .collect();
    nvim.input(&input).expect("Error run input command to nvim");
}

pub fn gtk_key_press(nvim: &mut Neovim, ev: &EventKey) -> Inhibit {
    if let Some(input) = convert_key(ev) {
        debug!("nvim_input -> {}", input);
        nvim.input(&input).expect("Error run input command to nvim");
        Inhibit(true)
    } else {
        Inhibit(false)
    }
}
