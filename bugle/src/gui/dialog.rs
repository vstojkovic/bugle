use std::cell::Cell;
use std::rc::Rc;

use fltk::button::Button;
use fltk::frame::Frame;
use fltk::prelude::*;
use fltk::window::Window;
use fltk_float::grid::{CellAlign, Grid, GridBuilder};

use super::wrapper_factory;

pub struct Dialog<T: Copy + 'static> {
    window: Window,
    result: Rc<Cell<Option<T>>>,
}

impl<T: Copy + 'static> Dialog<T> {
    pub fn default(
        parent: &impl WindowExt,
        title: &str,
        message: &str,
        choices: &[(&str, T)],
    ) -> Self {
        Self::new(parent, title, message, 480, 135, choices)
    }

    pub fn new(
        parent: &impl WindowExt,
        title: &str,
        message: &str,
        width: i32,
        height: i32,
        choices: &[(&str, T)],
    ) -> Self {
        let num_choices = choices.len();

        let mut window = Window::default().with_size(width, height).with_label(title);

        let mut grid = GridBuilder::with_factory(window.clone(), wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10)
            .with_padding(10, 10, 10, 10);
        grid.col().with_stretch(1).add();

        grid.row()
            .with_stretch(1)
            .with_default_align(CellAlign::Stretch)
            .add();
        grid.cell()
            .unwrap()
            .wrap(Frame::default_fill())
            .with_label(message);

        grid.row().add();

        let mut btn_grid = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10);
        btn_grid.row().add();
        let btn_group = btn_grid.col_group().add();
        btn_grid.extend_group(btn_group).batch(choices.len());

        let result = Rc::new(Cell::new(None));
        let mut buttons = Vec::with_capacity(num_choices);
        for (label, choice) in choices {
            let mut button = btn_grid
                .cell()
                .unwrap()
                .wrap(Button::default_fill())
                .with_label(label);
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

        let btn_grid = btn_grid.end();
        grid.cell()
            .unwrap()
            .with_horz_align(CellAlign::Center)
            .add(btn_grid);

        grid.end().layout_children();

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

    pub fn run(&self) -> Option<T> {
        while self.shown() && !fltk::app::should_program_quit() {
            fltk::app::wait();
        }
        self.result()
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
