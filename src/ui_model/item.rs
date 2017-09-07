
use sys::pango as sys_pango;
use pango;

#[derive(Clone)]
pub struct Item {
    pub item: sys_pango::Item,
    pub glyphs: Option<pango::GlyphString>,
    pub ink_rect: Option<pango::Rectangle>,
    font: pango::Font,
}

impl Item {
    pub fn new(item: sys_pango::Item) -> Self {
        Item {
            font: item.analysis().font(),
            item,
            glyphs: None,
            ink_rect: None,
        }
    }

    pub fn update(&mut self, item: sys_pango::Item) {
        self.font = item.analysis().font();
        self.item = item;
        self.glyphs = None;
        self.ink_rect = None;
    }

    pub fn set_glyphs(&mut self, glyphs: pango::GlyphString) {
        let mut glyphs = glyphs;
        // FIXME: pango units
        let (ink_rect, _) = glyphs.extents(&self.font);
        self.ink_rect = Some(ink_rect);
        self.glyphs = Some(glyphs);
    }

    pub fn font(&self) -> &pango::Font {
        &self.font
    }

    pub fn analysis(&self) -> sys_pango::Analysis {
        self.item.analysis()
    }
}
