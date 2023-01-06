use std::rc::Rc;

use fltk::button::Button;
use fltk::enums::{FrameType, Shortcut};
use fltk::group::{Group, Tile};
use fltk::menu::{MenuButton, MenuFlag};
use fltk::prelude::*;
use fltk_table::{SmartTable, TableOpts};

use super::CleanupFn;
use super::{button_row_height, prelude::*, widget_col_width};

pub(super) struct ModManager {
    root: Group,
}

impl ModManager {
    pub fn new() -> Rc<Self> {
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
        available_list.set_col_width(0, 350);
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

        let activate_button = Button::default().with_label("@>");
        let deactivate_button = Button::default().with_label("@<");
        let move_top_button = Button::default().with_label("@#8>|");
        let move_up_button = Button::default().with_label("@#8>");
        let move_down_button = Button::default().with_label("@#2>");
        let move_bottom_button = Button::default().with_label("@#2>|");
        let mut more_info_button = MenuButton::default().with_label("\u{1f4dc}");
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

        let manager = Rc::new(Self { root });

        manager
    }

    pub fn show(&self) -> CleanupFn {
        let mut root = self.root.clone();
        root.show();

        Box::new(move || {
            root.hide();
        })
    }
}
