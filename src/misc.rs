use std::borrow::Cow;

use regex::Regex;
use percent_encoding::percent_decode;

/// Escape special ASCII characters with a backslash.
pub fn escape_filename<'t>(filename: &'t str) -> Cow<'t, str> {
    lazy_static! {
        static ref SPECIAL_CHARS: Regex = if cfg!(target_os = "windows") {
            // On Windows, don't escape `:` and `\`, as these are valid components of the path.
            Regex::new(r"[[:ascii:]&&[^0-9a-zA-Z._:\\-]]").unwrap()
        } else {
            // Similarly, don't escape `/` on other platforms.
            Regex::new(r"[[:ascii:]&&[^0-9a-zA-Z._/-]]").unwrap()
        };
    }
    SPECIAL_CHARS.replace_all(&*filename, r"\$0")
}

/// Decode a file URI.
///
///   - On UNIX: `file:///path/to/a%20file.ext` -> `/path/to/a file.ext`
///   - On Windows: `file:///C:/path/to/a%20file.ext` -> `C:\path\to\a file.ext`
pub fn decode_uri(uri: &str) -> Option<String> {
    let path = match uri.split_at(8) {
        ("file:///", path) => path,
        _ => return None,
    };
    let path = percent_decode(path.as_bytes()).decode_utf8().ok()?;
    if cfg!(target_os = "windows") {
        lazy_static! {
            static ref SLASH: Regex = Regex::new(r"/").unwrap();
        }
        Some(String::from(SLASH.replace_all(&*path, r"\")))
    } else {
        Some("/".to_owned() + &path)
    }
}
