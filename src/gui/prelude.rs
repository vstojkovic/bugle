use fltk::prelude::*;

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
        self.with_pos(widget.x() + dx, widget.y() + dy)
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
