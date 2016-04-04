
use gdk;
use gdk::EventKey;
use phf;

include!(concat!(env!("OUT_DIR"), "/key_map_table.rs"));


fn keyval_to_input_string(val: &str, state: gdk::ModifierType) -> String {
    let mut input = String::from("<");
    if state.contains(gdk::enums::modifier_type::ShiftMask) {
        input.push_str("S-");
    }
    if state.contains(gdk::enums::modifier_type::ControlMask) {
        input.push_str("C-");
    }
    if state.contains(gdk::enums::modifier_type::Mod1Mask) {
        input.push_str("A-");
    }
    input.push_str(val);
    input.push_str(">");
    input
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
        return Some(if !state.is_empty() {
            keyval_to_input_string(&ch.to_string(), state)
        } else {
            ch.to_string()
        });
    }

    None
}
