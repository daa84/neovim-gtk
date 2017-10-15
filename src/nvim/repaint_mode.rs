use ui_model::{ModelRect, ModelRectVec};

#[derive(Clone, Debug)]
pub enum RepaintMode {
    Nothing,
    All,
    AreaList(ModelRectVec),
    Area(ModelRect),
}

impl RepaintMode {
    pub fn join(self, mode: RepaintMode) -> RepaintMode {
        match (self, mode) {
            (RepaintMode::Nothing, m) => m,
            (m, RepaintMode::Nothing) => m,
            (RepaintMode::All, _) => RepaintMode::All,
            (_, RepaintMode::All) => RepaintMode::All,
            (RepaintMode::Area(mr1), RepaintMode::Area(mr2)) => {
                let mut vec = ModelRectVec::new(mr1);
                vec.join(&mr2);
                RepaintMode::AreaList(vec)
            }
            (RepaintMode::AreaList(mut target), RepaintMode::AreaList(source)) => {
                for s in &source.list {
                    target.join(s);
                }
                RepaintMode::AreaList(target)
            }
            (RepaintMode::AreaList(mut list), RepaintMode::Area(l2)) => {
                list.join(&l2);
                RepaintMode::AreaList(list)
            }
            (RepaintMode::Area(l1), RepaintMode::AreaList(mut list)) => {
                list.join(&l1);
                RepaintMode::AreaList(list)
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode() {
        let mode = RepaintMode::Area(ModelRect::point(1, 1));
        let mode = mode.join(RepaintMode::Nothing);

        match mode {
            RepaintMode::Area(ref rect) => {
                assert_eq!(1, rect.top);
                assert_eq!(1, rect.bot);
                assert_eq!(1, rect.left);
                assert_eq!(1, rect.right);
            }
            _ => panic!("mode is worng"),
        }
    }
}
