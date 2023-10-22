use std::sync::Arc;

use fltk::browser::CheckBrowser;
use fltk::button::{Button, ReturnButton};
use fltk::frame::Frame;
use fltk::prelude::*;
use fltk::window::Window;
use fltk_float::grid::{CellAlign, Grid, GridBuilder};
use fltk_float::SimpleWrapper;

use crate::game::{ModRef, Mods};
use crate::gui::wrapper_factory;

pub struct ModUpdateSelectionDialog {
    window: Window,
    outdated_mods: Vec<ModRef>,
    mod_selection: CheckBrowser,
}

impl ModUpdateSelectionDialog {
    pub fn new(parent: &Window, mods: &Arc<Mods>, outdated_mods: Vec<ModRef>) -> Self {
        let mut window = Window::default()
            .with_size(480, 480)
            .with_label("Update Mods");

        let mut grid = GridBuilder::with_factory(window.clone(), wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10)
            .with_padding(10, 10, 10, 10);
        grid.col().with_stretch(1).add();

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(Frame::default_fill())
            .with_label(MSG_MODS_NEED_UPDATES);

        grid.row()
            .with_stretch(1)
            .with_default_align(CellAlign::Stretch)
            .add();
        let mut mod_selection = CheckBrowser::default();
        for mod_ref in outdated_mods.iter() {
            let mod_info = mods.get(mod_ref).unwrap();
            mod_selection.add(&mod_info.name, true);
        }
        grid.cell().unwrap().add(SimpleWrapper::new(
            mod_selection.clone(),
            Default::default(),
        ));

        let mut btn_grid = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10);
        btn_grid.row().add();
        let btn_group = btn_grid.col_group().add();

        btn_grid.extend_group(btn_group).batch(2);
        let mut btn_select_all = btn_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Select All");
        let mut btn_select_none = btn_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Select None");

        btn_grid.col().with_stretch(1).add();
        btn_grid.cell().unwrap().skip();

        btn_grid.extend_group(btn_group).add();
        let mut btn_proceed = btn_grid
            .cell()
            .unwrap()
            .wrap(ReturnButton::default())
            .with_label("Proceed");
        let btn_grid = btn_grid.end();

        grid.row().add();
        grid.cell().unwrap().add(btn_grid);

        grid.end().layout_children();

        btn_select_all.set_callback({
            let mut mod_selection = mod_selection.clone();
            move |_| mod_selection.check_all()
        });
        btn_select_none.set_callback({
            let mut mod_selection = mod_selection.clone();
            move |_| mod_selection.check_none()
        });
        btn_proceed.set_callback({
            let mut window = window.clone();
            move |_| window.hide()
        });

        window.set_pos(
            parent.x() + (parent.w() - window.w()) / 2,
            parent.y() + (parent.h() - window.h()) / 2,
        );

        Self {
            window,
            outdated_mods,
            mod_selection,
        }
    }

    pub fn run(self) -> Option<Vec<ModRef>> {
        let mut window = self.window.clone();
        window.make_modal(true);
        window.show();

        while window.shown() {
            if !fltk::app::wait() {
                return None;
            }
        }

        let mut result = Vec::new();
        for (idx, mod_ref) in self.outdated_mods.into_iter().enumerate() {
            if self.mod_selection.checked((idx + 1) as _) {
                result.push(mod_ref);
            }
        }
        Some(result)
    }
}

const MSG_MODS_NEED_UPDATES: &str = "The following mods in your mod list need to be updated:";
