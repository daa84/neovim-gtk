use std::str::CharIndices;

pub struct ItemizeIterator<'a> {
    char_iter: CharIndices<'a>,
    line: &'a str,
    prev_char: Option<(usize, char)>,
}

impl<'a> ItemizeIterator<'a> {
    pub fn new(line: &'a str) -> Self {
        ItemizeIterator {
            char_iter: line.char_indices(),
            line,
            prev_char: None,
        }
    }
}

impl<'a> Iterator for ItemizeIterator<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let mut start_index = None;

        let end_index = loop {
            let cha = self.prev_char.or_else(|| self.char_iter.next());
            self.prev_char = None;
            if let Some((index, ch)) = cha {
                let is_whitespace = ch.is_whitespace();
                let is_ascii = ch.is_ascii();

                if start_index.is_none() && !is_whitespace {
                    start_index = Some(index);
                    if !is_ascii {
                        break index + ch.len_utf8();
                    }
                }
                if start_index.is_some() && (is_whitespace || !is_ascii) {
                    self.prev_char = cha;
                    break index;
                }
            } else {
                break self.line.len();
            }
        };

        if let Some(start_index) = start_index {
            Some((start_index, end_index - start_index))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iterator() {
        let mut iter = ItemizeIterator::new("Test  line ");

        assert_eq!(Some((0, 4)), iter.next());
        assert_eq!(Some((6, 4)), iter.next());
        assert_eq!(None, iter.next());
    }
}
