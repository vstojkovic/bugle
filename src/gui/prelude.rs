use fltk::prelude::*;

pub(super) trait WidgetConvenienceExt {
    fn set_activated(&mut self, activated: bool);
}

impl<T: WidgetExt> WidgetConvenienceExt for T {
    fn set_activated(&mut self, activated: bool) {
        if activated {
            self.activate()
        } else {
            self.deactivate()
        }
    }
}

pub(super) trait LayoutExt {
    fn inside_of<W: WidgetExt>(self, widget: &W, dx: i32, dy: i32) -> Self;
    fn inside_parent(self, dx: i32, dy: i32) -> Self;
    fn with_size_flex(self, width: i32, height: i32) -> Self;
    fn stretch_to_parent(
        self,
        horz_margin: impl Into<Option<i32>>,
        vert_margin: impl Into<Option<i32>>,
    ) -> Self;
}

impl<T: WidgetExt> LayoutExt for T {
    fn inside_of<W: WidgetExt>(self, widget: &W, dx: i32, dy: i32) -> Self {
        let ox = if dx >= 0 { widget.x() } else { widget.x() + widget.width() };
        let oy = if dy >= 0 { widget.y() } else { widget.y() + widget.height() };
        self.with_pos(ox + dx, oy + dy)
    }

    fn inside_parent(self, dx: i32, dy: i32) -> Self {
        let parent = self.parent().unwrap();
        self.inside_of(&parent, dx, dy)
    }

    fn with_size_flex(self, mut width: i32, mut height: i32) -> Self {
        if width <= 0 {
            width += self.w();
        }
        if height <= 0 {
            height += self.h();
        }
        self.with_size(width, height)
    }

    fn stretch_to_parent(
        self,
        horz_margin: impl Into<Option<i32>>,
        vert_margin: impl Into<Option<i32>>,
    ) -> Self {
        let parent = self.parent().unwrap();
        let width = if let Some(margin) = horz_margin.into() {
            parent.w() - self.x() + parent.x() - margin
        } else {
            self.w()
        };
        let height = if let Some(margin) = vert_margin.into() {
            parent.h() - self.y() + parent.y() - margin
        } else {
            self.h()
        };
        self.with_size(width, height)
    }
}

macro_rules! declare_weak_cb {
    () => {
        fn weak_cb<A, C: FnMut(&Self) + 'static>(self: &Rc<Self>, mut cb: C) -> impl FnMut(&mut A) {
            let this = Rc::downgrade(self);
            move |_| {
                if let Some(this) = this.upgrade() {
                    cb(&*this)
                }
            }
        }
    };
}

pub(super) use declare_weak_cb;
