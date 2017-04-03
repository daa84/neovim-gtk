
use gdk;
use gdk::EventKey;
use phf;

use std::ascii::AsciiExt;

include!(concat!(env!("OUT_DIR"), "/key_map_table.rs"));


pub fn keyval_to_input_string(in_str: &str, in_state: gdk::ModifierType) -> String {
    let mut val = in_str;
    let mut state = in_state;
    let mut input = String::new();

    debug!("keyval -> {}", in_str);

    // CTRL-^ and CTRL-@ don't work in the normal way.
    if state.contains(gdk::CONTROL_MASK) && !state.contains(gdk::SHIFT_MASK) &&
       !state.contains(gdk::MOD1_MASK) {
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
            state.remove(gdk::SHIFT_MASK);
        }
    }

    if val == "<" {
        val = "lt";
    }

    if state.contains(gdk::SHIFT_MASK) {
        input.push_str("S-");
    }
    if state.contains(gdk::CONTROL_MASK) {
        input.push_str("C-");
    }
    if state.contains(gdk::MOD1_MASK) {
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
        Some(if !state.is_empty() {
            keyval_to_input_string(&ch.to_string(), state)
        } else {
            ch.to_string()
        })
    } else {
        None
    }
}
