use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use bit_vec::BitVec;
use fltk::button::Button;
use fltk::enums::{FrameType, Shortcut};
use fltk::group::{Group, Tile};
use fltk::menu::{MenuButton, MenuFlag};
use fltk::prelude::*;
use fltk_table::{SmartTable, TableOpts};

use crate::game::ModInfo;

use super::{alert_error, CleanupFn, Handler};
use super::{button_row_height, prelude::*, widget_col_width};

pub enum ModManagerAction {
    LoadModList,
}

pub enum ModManagerUpdate {
    PopulateModList {
        installed_mods: Arc<Vec<ModInfo>>,
        active_mods: Vec<usize>,
    },
}

#[derive(Default)]
struct ModListState {
    installed: Arc<Vec<ModInfo>>,
    available: Vec<usize>,
    active: Vec<usize>,
}

pub(super) struct ModManager {
    root: Group,
    on_action: Box<dyn Handler<ModManagerAction>>,
    available_list: SmartTable,
    active_list: SmartTable,
    state: RefCell<ModListState>,
}

impl ModManager {
    pub fn new(on_action: impl Handler<ModManagerAction> + 'static) -> Rc<Self> {
        let mut root = Group::default_fill();

        let tiles = Tile::default_fill();

        let mut tile_limits = Group::default_fill();
        tile_limits.end();
        tile_limits.hide();

        let left_tile = Group::default_fill()
            .inside_parent(0, 0)
            .with_size_flex(tiles.width() / 2, 0);

        let mut available_list = SmartTable::default_fill().with_opts(TableOpts {
            rows: 0,
            cols: 3,
            editable: false,
            ..Default::default()
        });
        available_list.make_resizable(true);
        available_list.set_row_header(false);
        available_list.set_col_resize(true);
        available_list.set_col_header_value(0, "Available Mods");
        available_list.set_col_width(0, 345);
        available_list.set_col_header_value(1, "Version");
        available_list.set_col_header_value(2, "Author");

        left_tile.end();

        let mut mid_tile = Group::default_fill().right_of(&left_tile, 0);
        mid_tile.set_frame(FrameType::FlatBox);

        let mut button_col = Group::default_fill();
        button_col.make_resizable(false);
        button_col.set_frame(FrameType::FlatBox);

        let mut button_group = Group::default();
        button_group.make_resizable(true);
        button_group.set_frame(FrameType::FlatBox);

        let activate_button = make_button("@>");
        let deactivate_button = make_button("@<");
        let move_top_button = make_button("@#8>|");
        let move_up_button = make_button("@#8>");
        let move_down_button = make_button("@#2>");
        let move_bottom_button = make_button("@#2>|");
        let mut more_info_button = MenuButton::default().with_label("\u{1f4dc}");
        more_info_button.deactivate();
        more_info_button.add("Description", Shortcut::None, MenuFlag::Normal, |_| ());
        more_info_button.add("Change Notes", Shortcut::None, MenuFlag::Normal, |_| ());

        let button_width = widget_col_width(&[
            &activate_button,
            &deactivate_button,
            &move_top_button,
            &move_up_button,
            &move_down_button,
            &move_bottom_button,
        ]);
        let button_height = button_row_height(&[
            &activate_button,
            &deactivate_button,
            &move_top_button,
            &move_up_button,
            &move_down_button,
            &move_bottom_button,
        ]);

        let mut mid_tile = mid_tile.with_size_flex(button_width + 20, 0);
        let button_col = button_col
            .inside_parent(0, 0)
            .with_size_flex(button_width + 20, 0)
            .stretch_to_parent(0, 0);
        let button_group = button_group.with_size(button_width, button_height * 9 + 40);
        let button_group = button_group.center_of_parent();
        let activate_button = activate_button
            .inside_parent(0, 0)
            .with_size(button_width, button_height);
        let deactivate_button = deactivate_button
            .below_of(&activate_button, 10)
            .with_size(button_width, button_height);
        let move_top_button = move_top_button
            .below_of(&deactivate_button, button_height)
            .with_size(button_width, button_height);
        let move_up_button = move_up_button
            .below_of(&move_top_button, 10)
            .with_size(button_width, button_height);
        let move_down_button = move_down_button
            .below_of(&move_up_button, 10)
            .with_size(button_width, button_height);
        let move_bottom_button = move_bottom_button
            .below_of(&move_down_button, 10)
            .with_size(button_width, button_height);
        let _more_info_button = more_info_button
            .below_of(&move_bottom_button, button_height)
            .with_size(button_width, button_height);

        button_group.end();
        button_col.end();

        mid_tile.end();

        let right_tile = Group::default_fill()
            .right_of(&mid_tile, 0)
            .stretch_to_parent(0, 0);

        let mut active_list = SmartTable::default_fill().with_opts(TableOpts {
            rows: 0,
            cols: 3,
            editable: false,
            ..Default::default()
        });
        active_list.make_resizable(true);
        active_list.set_row_header(false);
        active_list.set_col_resize(true);
        active_list.set_col_header_value(0, "Active Mods");
        active_list.set_col_width(0, 300);
        active_list.set_col_header_value(1, "Version");
        active_list.set_col_header_value(2, "Author");

        right_tile.end();

        let tile_limits = tile_limits
            .inside_parent(button_col.width() + 20, 0)
            .stretch_to_parent(button_col.width() + 20, 0);
        tiles.resizable(&tile_limits);

        tiles.end();

        root.end();
        root.hide();

        {
            let fixed_width = mid_tile.width();
            let tiles = tiles.clone();
            let mut left_tile = left_tile.clone();
            let mut right_tile = right_tile.clone();
            let mut old_x = mid_tile.x();
            mid_tile.resize_callback(move |tile, mut x, y, w, h| {
                if w == fixed_width {
                    return;
                }
                if x != old_x {
                    let rx = x + fixed_width;
                    let rw = tiles.x() + tiles.w() - rx;
                    right_tile.resize(rx, right_tile.y(), rw, right_tile.h());
                } else {
                    x = x + w - fixed_width;
                    let lw = x - left_tile.x();
                    left_tile.resize(left_tile.x(), left_tile.y(), lw, left_tile.h());
                }
                old_x = x;
                tile.resize(old_x, y, fixed_width, h);
            });
        }

        let manager = Rc::new(Self {
            root,
            on_action: Box::new(on_action),
            available_list,
            active_list,
            state: Default::default(),
        });

        manager
    }

    pub fn show(&self) -> CleanupFn {
        let mut root = self.root.clone();
        root.show();

        if let Err(err) = (self.on_action)(ModManagerAction::LoadModList) {
            alert_error(ERR_LOADING_MOD_LIST, &err);
        }

        Box::new(move || {
            root.hide();
        })
    }

    pub fn handle_update(&self, update: ModManagerUpdate) {
        match update {
            ModManagerUpdate::PopulateModList {
                installed_mods,
                active_mods,
            } => {
                self.populate_state(installed_mods, active_mods);
                self.populate_tables();
            }
        }
    }

    fn populate_state(&self, installed_mods: Arc<Vec<ModInfo>>, active_mods: Vec<usize>) {
        let mod_count = installed_mods.len();

        let mut state = self.state.borrow_mut();
        state.installed = installed_mods;
        state.available = Vec::with_capacity(mod_count);
        state.active = Vec::with_capacity(mod_count);

        let mut available_set = BitVec::from_elem(mod_count, true);
        for mod_idx in active_mods {
            available_set.set(mod_idx, false);
            state.active.push(mod_idx);
        }

        for mod_idx in 0..mod_count {
            if available_set[mod_idx] {
                state.available.push(mod_idx);
            }
        }
    }

    fn populate_tables(&self) {
        let state = self.state.borrow();

        populate_table(
            &mut self.available_list.clone(),
            &state.installed,
            &state.available,
        );
        populate_table(
            &mut self.active_list.clone(),
            &state.installed,
            &state.active,
        );
    }
}

const ERR_LOADING_MOD_LIST: &str = "Error while loading the mod list.";

fn make_button(text: &str) -> Button {
    let mut button = Button::default().with_label(text);
    button.deactivate();
    button
}

fn populate_table(table: &mut SmartTable, mods: &Vec<ModInfo>, indices: &Vec<usize>) {
    let mut rows = Vec::with_capacity(mods.len());
    for idx in indices {
        rows.push(make_mod_row(&mods[*idx]));
    }
    *table.data_ref().lock().unwrap() = rows;
    table.set_rows(indices.len() as _);
    table.redraw();
}

fn make_mod_row(mod_info: &ModInfo) -> Vec<String> {
    vec![
        mod_info.name.clone(),
        mod_info.version.to_string(),
        mod_info.author.clone(),
    ]
}
