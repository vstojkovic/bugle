use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use bbscope::BBCode;
use bit_vec::BitVec;
use fltk::app;
use fltk::button::Button;
use fltk::enums::{Event, FrameType, Shortcut};
use fltk::group::{Group, Tile};
use fltk::menu::{MenuButton, MenuFlag};
use fltk::prelude::*;
use fltk::table::TableContext;
use fltk::window::Window;
use fltk_table::{SmartTable, TableOpts};
use fltk_webview::Webview;
use lazy_static::lazy_static;

use crate::game::ModInfo;

use super::{alert_error, CleanupFn, Handler};
use super::{button_row_height, prelude::*, widget_col_width};

pub enum ModManagerAction {
    LoadModList,
    SaveModList {
        installed_mods: Arc<Vec<ModInfo>>,
        active_mods: Vec<usize>,
    },
}

pub enum ModManagerUpdate {
    PopulateModList {
        installed_mods: Arc<Vec<ModInfo>>,
        active_mods: Vec<usize>,
    },
}

enum Selection {
    Available(usize),
    Active(usize),
}

#[derive(Default)]
struct ModListState {
    installed: Arc<Vec<ModInfo>>,
    available: Vec<usize>,
    active: Vec<usize>,
    selection: Option<Selection>,
}

impl ModListState {
    fn get_selected_available(&self) -> Option<usize> {
        if let Some(Selection::Available(idx)) = self.selection {
            Some(idx)
        } else {
            None
        }
    }

    fn get_selected_active(&self) -> Option<usize> {
        if let Some(Selection::Active(idx)) = self.selection {
            Some(idx)
        } else {
            None
        }
    }

    fn selected_mod_info(&self) -> Option<&ModInfo> {
        match self.selection {
            None => None,
            Some(Selection::Available(idx)) => Some(&self.installed[self.available[idx]]),
            Some(Selection::Active(idx)) => Some(&self.installed[self.active[idx]]),
        }
    }
}

pub(super) struct ModManager {
    root: Group,
    on_action: Box<dyn Handler<ModManagerAction>>,
    available_list: SmartTable,
    active_list: SmartTable,
    activate_button: Button,
    deactivate_button: Button,
    move_top_button: Button,
    move_up_button: Button,
    move_down_button: Button,
    move_bottom_button: Button,
    more_info_button: MenuButton,
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

        let button_width = widget_col_width(&[
            &activate_button,
            &deactivate_button,
            &move_top_button,
            &move_up_button,
            &move_down_button,
            &move_bottom_button,
            &more_info_button,
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
        let mut activate_button = activate_button
            .inside_parent(0, 0)
            .with_size(button_width, button_height);
        let mut deactivate_button = deactivate_button
            .below_of(&activate_button, 10)
            .with_size(button_width, button_height);
        let mut move_top_button = move_top_button
            .below_of(&deactivate_button, button_height)
            .with_size(button_width, button_height);
        let mut move_up_button = move_up_button
            .below_of(&move_top_button, 10)
            .with_size(button_width, button_height);
        let mut move_down_button = move_down_button
            .below_of(&move_up_button, 10)
            .with_size(button_width, button_height);
        let mut move_bottom_button = move_bottom_button
            .below_of(&move_down_button, 10)
            .with_size(button_width, button_height);
        let mut more_info_button = more_info_button
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
            available_list: available_list.clone(),
            active_list: active_list.clone(),
            activate_button: activate_button.clone(),
            deactivate_button: deactivate_button.clone(),
            move_top_button: move_top_button.clone(),
            move_up_button: move_up_button.clone(),
            move_down_button: move_down_button.clone(),
            move_bottom_button: move_bottom_button.clone(),
            more_info_button: more_info_button.clone(),
            state: Default::default(),
        });

        available_list.set_callback(manager.weak_cb(|this| {
            if (app::event() == Event::Released)
                && app::event_is_click()
                && this.available_list.callback_context() == TableContext::Cell
            {
                this.available_clicked();
            }
        }));

        active_list.set_callback(manager.weak_cb(|this| {
            if (app::event() == Event::Released)
                && app::event_is_click()
                && this.active_list.callback_context() == TableContext::Cell
            {
                this.active_clicked();
            }
        }));

        activate_button.set_callback(manager.weak_cb(Self::activate_clicked));
        deactivate_button.set_callback(manager.weak_cb(Self::deactivate_clicked));
        move_top_button.set_callback(manager.weak_cb(Self::move_top_clicked));
        move_up_button.set_callback(manager.weak_cb(Self::move_up_clicked));
        move_down_button.set_callback(manager.weak_cb(Self::move_down_clicked));
        move_bottom_button.set_callback(manager.weak_cb(Self::move_bottom_clicked));

        more_info_button.add(
            "Description",
            Shortcut::None,
            MenuFlag::Normal,
            manager.weak_cb(Self::show_description),
        );
        more_info_button.add(
            "Change Notes",
            Shortcut::None,
            MenuFlag::Normal,
            manager.weak_cb(Self::show_change_notes),
        );

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

    declare_weak_cb!();

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

    fn available_clicked(&self) {
        let mut table = self.available_list.clone();
        let _ = table.take_focus();

        self.set_selection(Some(Selection::Available(table.callback_row() as _)));
    }

    fn active_clicked(&self) {
        let mut table = self.active_list.clone();
        let _ = table.take_focus();

        self.set_selection(Some(Selection::Active(table.callback_row() as _)));
    }

    fn set_selection(&self, selection: Option<Selection>) {
        let mut state = self.state.borrow_mut();
        state.selection = selection;
        match state.selection {
            None => {
                self.available_list.clone().unset_selection();
                self.active_list.clone().unset_selection();
            }
            Some(Selection::Available(_)) => self.active_list.clone().unset_selection(),
            Some(Selection::Active(_)) => self.available_list.clone().unset_selection(),
        }
        drop(state);
        self.update_actions();
    }

    fn update_actions(&self) {
        let state = self.state.borrow();
        let (activate, deactivate, move_up, move_down, more_info) = match state.selection {
            None => (false, false, false, false, false),
            Some(Selection::Available(_)) => (true, false, false, false, true),
            Some(Selection::Active(idx)) => {
                let last_idx = state.active.len() - 1;
                (false, true, idx > 0, idx < last_idx, true)
            }
        };

        self.activate_button.clone().set_activated(activate);
        self.deactivate_button.clone().set_activated(deactivate);
        self.move_top_button.clone().set_activated(move_up);
        self.move_up_button.clone().set_activated(move_up);
        self.move_down_button.clone().set_activated(move_down);
        self.move_bottom_button.clone().set_activated(move_down);
        self.more_info_button.clone().set_activated(more_info);
    }

    fn activate_clicked(&self) {
        let mut state = self.state.borrow_mut();
        let row_idx = state.get_selected_available().unwrap();

        let mod_idx = state.available.remove(row_idx);
        state.active.push(mod_idx);

        let row = mutate_table(&mut self.available_list.clone(), |data| {
            data.remove(row_idx)
        });
        mutate_table(&mut self.active_list.clone(), |data| data.push(row));

        drop(state);

        self.set_selection(None);
        self.save_mod_list();
    }

    fn deactivate_clicked(&self) {
        let mut state = self.state.borrow_mut();
        let row_idx = state.get_selected_active().unwrap();

        let mod_idx = state.active.remove(row_idx);
        let dest_row_idx = state.available.binary_search(&mod_idx).unwrap_err();
        state.available.insert(dest_row_idx, mod_idx);

        let row = mutate_table(&mut self.active_list.clone(), |data| data.remove(row_idx));
        mutate_table(&mut self.available_list.clone(), |data| {
            data.insert(dest_row_idx, row)
        });

        drop(state);

        self.set_selection(None);
        self.save_mod_list();
    }

    fn move_top_clicked(&self) {
        let mut state = self.state.borrow_mut();
        let row_idx = state.get_selected_active().unwrap();
        state.active[0..(row_idx + 1)].rotate_right(1);

        let mut active_list = self.active_list.clone();
        mutate_table(&mut active_list, |data| {
            data[0..(row_idx + 1)].rotate_right(1)
        });
        let (_, left, _, right) = active_list.get_selection();
        active_list.set_selection(0, left, 0, right);

        drop(state);

        self.set_selection(Some(Selection::Active(0)));
        self.save_mod_list();
    }

    fn move_up_clicked(&self) {
        let mut state = self.state.borrow_mut();
        let row_idx = state.get_selected_active().unwrap();
        state.active.swap(row_idx - 1, row_idx);

        let mut active_list = self.active_list.clone();
        mutate_table(&mut active_list, |data| data.swap(row_idx - 1, row_idx));
        let (_, left, _, right) = active_list.get_selection();
        active_list.set_selection((row_idx - 1) as _, left, (row_idx - 1) as _, right);

        drop(state);

        self.set_selection(Some(Selection::Active(row_idx - 1)));
        self.save_mod_list();
    }

    fn move_down_clicked(&self) {
        let mut state = self.state.borrow_mut();
        let row_idx = state.get_selected_active().unwrap();
        state.active.swap(row_idx + 1, row_idx);

        let mut active_list = self.active_list.clone();
        mutate_table(&mut active_list, |data| data.swap(row_idx + 1, row_idx));
        let (_, left, _, right) = active_list.get_selection();
        active_list.set_selection((row_idx + 1) as _, left, (row_idx + 1) as _, right);

        drop(state);

        self.set_selection(Some(Selection::Active(row_idx + 1)));
        self.save_mod_list();
    }

    fn move_bottom_clicked(&self) {
        let mut state = self.state.borrow_mut();
        let row_idx = state.get_selected_active().unwrap();
        state.active[row_idx..].rotate_left(1);

        let mut active_list = self.active_list.clone();
        mutate_table(&mut active_list, |data| data[row_idx..].rotate_left(1));
        let last_idx = state.active.len() - 1;
        let (_, left, _, right) = active_list.get_selection();
        active_list.set_selection(last_idx as _, left, last_idx as _, right);

        drop(state);

        self.set_selection(Some(Selection::Active(last_idx)));
        self.save_mod_list();
    }

    fn save_mod_list(&self) {
        let state = self.state.borrow();
        if let Err(err) = (self.on_action)(ModManagerAction::SaveModList {
            installed_mods: Arc::clone(&state.installed),
            active_mods: state.active.clone(),
        }) {
            alert_error(ERR_SAVING_MOD_LIST, &err);
        }
    }

    fn show_description(&self) {
        let state = self.state.borrow();
        let mod_info = state.selected_mod_info().unwrap();
        self.show_bbcode(
            &format!("Description: {}", &mod_info.name),
            &mod_info.description,
        );
    }

    fn show_change_notes(&self) {
        let state = self.state.borrow();
        let mod_info = state.selected_mod_info().unwrap();
        self.show_bbcode(
            &format!("Change Notes: {}", &mod_info.name),
            &mod_info.change_notes,
        );
    }

    fn show_bbcode(&self, title: &str, content: &str) {
        let mut html = BBCODE.parse(content);
        html = html.replace("\n", "<br>");
        html = format!(
            "<html><head><style>{}</style></head><body>{}</body></html",
            CSS_INFO_BODY, html
        );
        html = urlencoding::encode(&html).to_string();

        let mut popup = Window::default().with_label(title).with_size(800, 600);
        popup.make_modal(true);
        popup.make_resizable(true);
        popup.end();
        popup.show();

        let webview = Webview::create(false, &mut popup);
        webview.set_html(&html);

        while popup.shown() {
            app::wait();
        }
    }
}

const ERR_LOADING_MOD_LIST: &str = "Error while loading the mod list.";
const ERR_SAVING_MOD_LIST: &str = "Error while saving the mod list.";
const CSS_INFO_BODY: &str = include_str!("mod_info.css");

lazy_static! {
    static ref BBCODE: BBCode = {
        use bbscope::MatchType;

        let mut matchers = BBCode::basics().unwrap();
        matchers.append(&mut BBCode::extras().unwrap());

        for matcher in matchers.iter_mut() {
            if matcher.id == "url" {
                if let MatchType::Open(ref mut info) = matcher.match_type {
                    if let Some(ref mut only) = Arc::get_mut(info).unwrap().only {
                        only.push("img");
                    }
                }
            }
        }

        BBCode::from_matchers(matchers)
    };
}

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

fn mutate_table<R>(table: &mut SmartTable, mutator: impl FnOnce(&mut Vec<Vec<String>>) -> R) -> R {
    let data_ref = table.data_ref();
    let mut data = data_ref.lock().unwrap();
    let result = mutator(&mut data);
    table.set_rows(data.len() as _);
    table.redraw();
    result
}
