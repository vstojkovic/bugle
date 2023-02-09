use std::rc::Rc;
use std::sync::Arc;

use fltk::button::Button;
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::misc::InputChoice;
use fltk::prelude::*;
use fltk_table::{SmartTable, TableOpts};

use crate::game::MapInfo;

use super::prelude::LayoutExt;
use super::{button_row_height, widget_auto_width, widget_col_width, CleanupFn};

pub struct SinglePlayer {
    root: Group,
}

impl SinglePlayer {
    pub fn new(maps: Arc<Vec<MapInfo>>) -> Rc<Self> {
        let mut root = Group::default_fill();

        let map_label = Frame::default().with_label("Map:");
        let label_width = widget_auto_width(&map_label);

        let new_button = Button::default().with_label("New");
        let continue_button = Button::default().with_label("Continue");
        let load_button = Button::default().with_label("Load");
        let save_button = Button::default().with_label("Save");
        let save_as_button = Button::default().with_label("Save As...");
        let delete_button = Button::default().with_label("Delete");

        let button_width = widget_col_width(&[
            &new_button,
            &continue_button,
            &load_button,
            &save_button,
            &save_as_button,
            &delete_button,
        ]);
        let row_height = button_row_height(&[
            &new_button,
            &continue_button,
            &load_button,
            &save_button,
            &save_as_button,
            &delete_button,
        ]);

        let delete_button = delete_button
            .with_size(button_width, row_height)
            .inside_parent(-button_width, 0);
        let save_as_button = save_as_button
            .with_size(button_width, row_height)
            .left_of(&delete_button, 10);
        let save_button = save_button
            .with_size(button_width, row_height)
            .left_of(&save_as_button, 10);
        let load_button = load_button
            .with_size(button_width, row_height)
            .left_of(&save_button, 10);
        let continue_button = continue_button
            .with_size(button_width, row_height)
            .left_of(&load_button, 10);
        let new_button = new_button
            .with_size(button_width, row_height)
            .left_of(&continue_button, 10);

        let map_label = map_label
            .inside_parent(0, 0)
            .with_size(label_width, row_height);
        let map_input = InputChoice::default_fill().right_of(&map_label, 10);
        let map_input_width = new_button.x() - map_input.x() - 10;
        let mut map_input = map_input.with_size(map_input_width, row_height);
        for map in maps.iter() {
            map_input.add(&map.display_name);
        }
        map_input.input().set_readonly(true);
        map_input.input().clear_visible_focus();

        let db_pane = Group::default_fill()
            .below_of(&map_input, 10)
            .stretch_to_parent(0, 0);

        let mut db_list = SmartTable::default_fill().with_opts(TableOpts {
            rows: 2,
            cols: 5,
            ..Default::default()
        });
        db_list.make_resizable(true);
        db_list.set_col_resize(true);
        db_list.set_row_header_value(0, "In Progress");
        db_list.set_row_header_value(1, "Backup");
        db_list.set_row_header_width(100);
        db_list.set_col_header_value(0, "Filename");
        db_list.set_col_width(0, 310);
        db_list.set_col_header_value(1, "Last Played");
        db_list.set_col_width(1, 200);
        db_list.set_col_header_value(2, "Character");
        db_list.set_col_width(2, 160);
        db_list.set_col_header_value(3, "Level");
        db_list.set_col_width(3, 50);
        db_list.set_col_header_value(4, "Clan");
        db_list.set_col_width(4, 150);
        db_list.end();

        db_pane.end();

        root.end();
        root.hide();

        Rc::new(Self { root })
    }

    pub fn show(&self) -> CleanupFn {
        let mut root = self.root.clone();
        root.show();

        Box::new(move || {
            root.hide();
        })
    }
}
