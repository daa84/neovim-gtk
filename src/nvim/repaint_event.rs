use fnv::FnvHashMap;

use ui_model::{ModelRect, ModelRectVec};

pub struct RepaintEvent {
    pub events: FnvHashMap<u64, RepaintGridEvent>,
    pub repaint_all: bool,
}

impl RepaintEvent {
    pub fn new() -> Self {
        RepaintEvent {
            events: FnvHashMap::default(),
            repaint_all: false,
        }
    }

    pub fn all() -> Self {
        RepaintEvent {
            events: FnvHashMap::default(),
            repaint_all: true,
        }
    }

    pub fn from_grid_event(grid_id: u64, mode: RepaintMode) -> Self {
        let ev = RepaintEvent::new();
        ev.join(RepaintGridEvent::new(grid_id, mode));
        ev
    }

    pub fn join(&mut self, event: RepaintGridEvent) {
        match event.mode {
            RepaintMode::Nothing => return,
            RepaintMode::All => {
                if event.grid_id.is_none() {
                    self.repaint_all = true;
                    return;
                }
            }
            _ => (),
        }

        if !self.repaint_all {
            let grid_id = event.grid_id.unwrap();
            if self.events.contains_key(&grid_id) {
                let previsous_event = &mut self.events[&grid_id];
                previsous_event.join(event);
            } else {
                self.events.insert(grid_id, event);
            }
        }
    }
}

pub struct RepaintGridEvent {
    pub grid_id: Option<u64>,
    pub mode: RepaintMode,
}

impl RepaintGridEvent {
    pub fn new(grid_id: u64, mode: RepaintMode) -> Self {
        RepaintGridEvent {
            grid_id: Some(grid_id),
            mode,
        }
    }

    pub fn all() -> Self {
        RepaintGridEvent {
            grid_id: None,
            mode: RepaintMode::All,
        }
    }

    pub fn nothing() -> Self {
        RepaintGridEvent {
            grid_id: None,
            mode: RepaintMode::Nothing,
        }
    }

    pub fn join(&mut self, event: RepaintGridEvent) {
        self.mode.join(event.mode);
    }
}

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
