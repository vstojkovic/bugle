use fltk::prelude::*;

pub(super) trait WidgetConvenienceExt {
    fn set_activated(&mut self, activated: bool);
    fn with_tooltip(self, tooltip: &str) -> Self;
}

impl<T: WidgetExt> WidgetConvenienceExt for T {
    fn set_activated(&mut self, activated: bool) {
        if activated {
            self.activate()
        } else {
            self.deactivate()
        }
    }

    fn with_tooltip(mut self, tooltip: &str) -> Self {
        self.set_tooltip(tooltip);
        self
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
