use std::borrow::Cow;
use std::mem;

use lazy_static::lazy_static;

use percent_encoding::percent_decode;
use regex::Regex;

use crate::shell;

/// Split comma separated parameters with ',' except escaped '\\,'
pub fn split_at_comma(source: &str) -> Vec<String> {
    let mut items = Vec::new();

    let mut escaped = false;
    let mut item = String::new();

    for ch in source.chars() {
        if ch == ',' && !escaped {
            item = item.replace("\\,", ",");

            let mut new_item = String::new();
            mem::swap(&mut item, &mut new_item);

            items.push(new_item);
        } else {
            item.push(ch);
        }
        escaped = ch == '\\';
    }

    if !item.is_empty() {
        items.push(item.replace("\\,", ","));
    }

    items
}

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

/// info text
pub fn about_comments() -> String {
    format!(
        "Build on top of neovim\n\
         Minimum supported neovim version: {}",
        shell::MINIMUM_SUPPORTED_NVIM_VERSION
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comma_split() {
        let res = split_at_comma("a,b");
        assert_eq!(2, res.len());
        assert_eq!("a", res[0]);
        assert_eq!("b", res[1]);

        let res = split_at_comma("a,b\\,c");
        assert_eq!(2, res.len());
        assert_eq!("a", res[0]);
        assert_eq!("b,c", res[1]);
    }
}
