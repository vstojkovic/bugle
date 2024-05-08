use std::cell::RefCell;
use std::rc::Rc;

use dynabus::Bus;
use fltk::frame::Frame;
use fltk::misc::Progress;
use fltk::prelude::*;
use fltk::window::Window;
use fltk_float::grid::{CellAlign, GridBuilder};
use fltk_float::SimpleWrapper;

use crate::bus::AppBus;
use crate::gui::wrapper_factory;
use crate::util::weak_cb;

pub struct TaskProgressMonitor {
    bus: Rc<RefCell<AppBus>>,
    window: Window,
    progress_bar: Progress,
}

#[derive(dynabus::Event)]
pub enum TaskProgressUpdate {
    Running { done: f64, total: f64 },
    Stopped,
}

impl TaskProgressMonitor {
    pub fn default(
        bus: Rc<RefCell<AppBus>>,
        parent: &impl WindowExt,
        title: &str,
        message: &str,
    ) -> Rc<Self> {
        Self::new(bus, parent, title, message, 480, 135)
    }

    pub fn new(
        bus: Rc<RefCell<AppBus>>,
        parent: &impl WindowExt,
        title: &str,
        message: &str,
        width: i32,
        height: i32,
    ) -> Rc<Self> {
        let mut window = Window::default().with_size(width, height).with_label(title);

        let mut grid = GridBuilder::with_factory(window.clone(), wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10)
            .with_padding(10, 10, 10, 10);
        grid.col().with_stretch(1).add();

        grid.row()
            .with_stretch(1)
            .with_default_align(CellAlign::End)
            .add();
        grid.cell()
            .unwrap()
            .wrap(Frame::default_fill())
            .with_label(message);

        grid.row()
            .with_stretch(1)
            .with_default_align(CellAlign::Start)
            .add();
        let mut progress_bar = Progress::default();
        progress_bar.set_minimum(0.0);
        progress_bar.set_maximum(100.0);
        progress_bar.set_selection_color(fltk::enums::Color::Free);
        grid.cell()
            .unwrap()
            .with_horz_align(CellAlign::Stretch)
            .add(SimpleWrapper::new(
                progress_bar.clone(),
                fltk_float::Size {
                    width: 0,
                    height: 16,
                },
            ));

        grid.end().layout_children();

        window.set_pos(
            parent.x() + (parent.w() - window.w()) / 2,
            parent.y() + (parent.h() - window.h()) / 2,
        );

        let this = Rc::new(Self {
            bus: Rc::clone(&bus),
            window,
            progress_bar,
        });

        this
    }

    pub fn run(self: Rc<Self>) {
        let mut bus = self.bus.borrow_mut();
        let update_sub = bus.subscribe_consumer(weak_cb!([this = &self] => |update| {
            match update {
                TaskProgressUpdate::Running { done, total } => this.update_progress(done, total),
                TaskProgressUpdate::Stopped => this.window.clone().hide(),
            }
        }));
        drop(bus);

        let mut window = self.window.clone();
        window.make_modal(true);
        window.show();

        while window.shown() && !fltk::app::should_program_quit() {
            fltk::app::wait();
        }

        let mut bus = self.bus.borrow_mut();
        bus.unsubscribe(update_sub);
        drop(bus);
    }

    fn update_progress(&self, done: f64, total: f64) {
        let mut progress_bar = self.progress_bar.clone();
        progress_bar.set_maximum(total);
        progress_bar.set_value(done);
    }
}
