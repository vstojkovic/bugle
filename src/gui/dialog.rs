use std::cell::Cell;
use std::rc::Rc;

use fltk::button::Button;
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::prelude::*;
use fltk::window::Window;

use super::prelude::*;
use super::{button_auto_height, widget_auto_width};

pub struct Dialog<T: Copy + 'static> {
    window: Window,
    result: Rc<Cell<Option<T>>>,
}

impl<T: Copy + 'static> Dialog<T> {
    pub fn default(parent: &Window, title: &str, message: &str, choices: &[(&str, T)]) -> Self {
        Self::new(parent, title, message, 480, 135, choices)
    }

    pub fn new(
        parent: &Window,
        title: &str,
        message: &str,
        width: i32,
        height: i32,
        choices: &[(&str, T)],
    ) -> Self {
        let num_choices = choices.len();

        let mut window = Window::default().with_size(width, height).with_label(title);

        let button_row = Group::default_fill();
        let button_group = Group::default_fill();

        let result = Rc::new(Cell::new(None));
        let mut buttons = Vec::with_capacity(num_choices);
        for (label, choice) in choices {
            let mut button = Button::default_fill().with_label(label);
            button.set_callback({
                let result = Rc::clone(&result);
                let choice = *choice;
                let mut window = window.clone();
                move |_| {
                    result.set(Some(choice));
                    window.hide();
                }
            });
            buttons.push(button);
        }

        button_group.end();
        button_row.end();

        let btn_width = buttons
            .iter()
            .map(|btn| widget_auto_width(btn))
            .max()
            .unwrap();
        let btn_height = buttons
            .iter()
            .map(|btn| button_auto_height(btn))
            .max()
            .unwrap();
        let grp_width = (btn_width + 10) * (num_choices as i32) - 10;
        let grp_height = btn_height + 20;

        let button_row = button_row
            .with_size_flex(0, grp_height)
            .inside_parent(0, -grp_height);
        let _button_group = button_group
            .with_size(grp_width, grp_height)
            .center_of_parent();
        let mut prev_button = None;
        for button in buttons {
            let button = button.with_size(btn_width, btn_height);
            prev_button = Some(if let Some(prev) = prev_button {
                button.right_of(&prev, 10)
            } else {
                button.inside_parent(0, 0)
            });
        }

        Frame::default_fill()
            .with_label(message)
            .with_size_flex(0, button_row.y());

        window.end();
        window.set_pos(
            parent.x() + (parent.w() - window.w()) / 2,
            parent.y() + (parent.h() - window.h()) / 2,
        );

        Self { window, result }
    }

    pub fn show(&self) {
        let mut window = self.window.clone();
        window.make_modal(true);
        window.show();
    }

    pub fn result(&self) -> Option<T> {
        self.result.get()
    }

    pub fn shown(&self) -> bool {
        self.window.shown()
    }
}

impl<T: Copy + 'static> Drop for Dialog<T> {
    fn drop(&mut self) {
        self.window.hide();
    }
}
