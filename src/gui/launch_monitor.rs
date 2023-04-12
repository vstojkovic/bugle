use std::cell::Cell;
use std::rc::Rc;

use fltk::button::Button;
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::prelude::*;
use fltk::window::Window;

use super::prelude::*;
use super::{button_auto_height, widget_auto_width};

pub struct LaunchMonitor {
    window: Window,
    cancel_requested: Rc<Cell<bool>>,
}

impl LaunchMonitor {
    pub fn new(parent: &Window) -> Self {
        let mut window = Window::default()
            .with_size(320, 90)
            .with_label("Launching Conan Exiles");

        let cancel_group = Group::default_fill();
        let cancel_btn = Button::default_fill().with_label("Cancel");
        cancel_group.end();

        let btn_width = widget_auto_width(&cancel_btn);
        let btn_height = button_auto_height(&cancel_btn);
        let grp_height = btn_height + 20;
        let cancel_group = cancel_group
            .with_size_flex(0, grp_height)
            .inside_parent(0, -grp_height);
        let mut cancel_btn = cancel_btn
            .with_size(btn_width, btn_height)
            .center_of_parent();

        Frame::default_fill()
            .with_label("Waiting for Conan Exiles to start...")
            .with_size_flex(0, cancel_group.y());

        window.end();
        window.set_pos(
            parent.x() + (parent.w() - window.w()) / 2,
            parent.y() + (parent.h() - window.h()) / 2,
        );

        let cancel_flag = Rc::new(Cell::new(false));
        cancel_btn.set_callback({
            let cancel_flag = Rc::clone(&cancel_flag);
            move |_| {
                cancel_flag.set(true);
            }
        });

        Self {
            window,
            cancel_requested: cancel_flag,
        }
    }

    pub fn show(&self) {
        let mut window = self.window.clone();
        window.make_modal(true);
        window.show();
    }

    pub fn cancel_requested(&self) -> bool {
        self.cancel_requested.get()
    }
}

impl Drop for LaunchMonitor {
    fn drop(&mut self) {
        self.window.hide();
    }
}
