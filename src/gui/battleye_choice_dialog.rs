use std::cell::Cell;
use std::rc::Rc;

use fltk::button::Button;
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::prelude::*;
use fltk::window::Window;

use super::prelude::LayoutExt;
use super::{button_row_height, widget_col_width};

pub struct BattlEyeChoiceDialog {
    window: Window,
    result: Rc<Cell<Option<bool>>>,
}

impl BattlEyeChoiceDialog {
    pub fn new(parent: &Window) -> Self {
        let mut window = Window::default()
            .with_size(480, 135)
            .with_label("Enable BattlEye?");

        let button_row = Group::default_fill();
        let button_group = Group::default_fill();
        let enable_btn = Button::default_fill().with_label("Enable");
        let disable_btn = Button::default_fill().with_label("Disable");
        button_group.end();
        button_row.end();

        let btn_width = widget_col_width(&[&enable_btn, &disable_btn]);
        let btn_height = button_row_height(&[&enable_btn, &disable_btn]);
        let grp_width = btn_width * 2 + 10;
        let grp_height = btn_height + 20;

        let button_row = button_row
            .with_size_flex(0, grp_height)
            .inside_parent(0, -grp_height);
        let _button_group = button_group
            .with_size(grp_width, grp_height)
            .center_of_parent();
        let mut enable_btn = enable_btn
            .with_size(btn_width, btn_height)
            .inside_parent(0, 0);
        let mut disable_btn = disable_btn
            .with_size(btn_width, btn_height)
            .right_of(&enable_btn, 10);

        Frame::default_fill()
            .with_label(
                "BUGLE could not determine whether BattlEye is required for this session.\nStart Conan Exiles with BattlEye enabled or disabled?"
            )
            .with_size_flex(0, button_row.y());

        window.end();
        window.set_pos(
            parent.x() + (parent.w() - window.w()) / 2,
            parent.y() + (parent.h() - window.h()) / 2,
        );

        let result = Rc::new(Cell::new(None));
        enable_btn.set_callback({
            let result = Rc::clone(&result);
            move |_| result.set(Some(true))
        });
        disable_btn.set_callback({
            let result = Rc::clone(&result);
            move |_| result.set(Some(false))
        });

        Self { window, result }
    }

    pub fn show(&self) {
        let mut window = self.window.clone();
        window.make_modal(true);
        window.show();
    }

    pub fn result(&self) -> Option<bool> {
        self.result.get()
    }
}

impl Drop for BattlEyeChoiceDialog {
    fn drop(&mut self) {
        self.window.hide();
    }
}
