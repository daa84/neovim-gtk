use std::str::CharIndices;

pub struct ItemizeIterator<'a> {
    char_iter: CharIndices<'a>,
    line: &'a str,
}

impl<'a> ItemizeIterator<'a> {
    pub fn new(line: &'a str) -> Self {
        ItemizeIterator {
            char_iter: line.char_indices(),
            line,
        }
    }
}

impl<'a> Iterator for ItemizeIterator<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let mut start_index = None;

        let end_index = loop {
            if let Some((index, ch)) = self.char_iter.next() {
                let is_whitespace = ch.is_whitespace();

                if start_index.is_none() && !is_whitespace {
                    start_index = Some(index);
                }
                if start_index.is_some() && is_whitespace {
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
