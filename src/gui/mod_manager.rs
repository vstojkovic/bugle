use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use bbscope::{BBCode, BBCodeTagConfig};
use bit_vec::BitVec;
use fltk::app;
use fltk::button::Button;
use fltk::enums::{Align, FrameType, Shortcut};
use fltk::group::{Group, Tile};
use fltk::menu::{MenuButton, MenuFlag};
use fltk::prelude::*;
use fltk::table::TableContext;
use fltk::window::Window;
use fltk_float::grid::{CellAlign, Grid, GridBuilder};
use fltk_float::SimpleWrapper;
use fltk_webview::Webview;
use lazy_static::lazy_static;
use slog::{error, Logger};

use crate::game::{ModInfo, ModRef, Mods};

use super::prelude::*;
use super::widgets::{DataTable, DataTableProperties, DataTableUpdate};
use super::{alert_error, is_table_nav_event, prompt_confirm, wrapper_factory, CleanupFn, Handler};

pub enum ModManagerAction {
    LoadModList,
    SaveModList(Vec<ModRef>),
    ImportModList,
    ExportModList(Vec<ModRef>),
}

pub enum ModManagerUpdate {
    PopulateModList(Vec<ModRef>),
}

enum Selection {
    Available(usize),
    Active(usize),
}

struct ModListState {
    installed: Arc<Mods>,
    available: Vec<ModRef>,
    active: Vec<ModRef>,
    selection: Option<Selection>,
}

impl ModListState {
    fn new(mods: Arc<Mods>) -> Self {
        Self {
            installed: mods,
            available: Vec::new(),
            active: Vec::new(),
            selection: None,
        }
    }

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
            Some(Selection::Available(idx)) => self.installed.get(&self.available[idx]),
            Some(Selection::Active(idx)) => self.installed.get(&self.active[idx]),
        }
    }
}

type ModRow = [String; 3];

pub(super) struct ModManager {
    logger: Logger,
    tiles: Tile,
    on_action: Box<dyn Handler<ModManagerAction>>,
    available_list: DataTable<ModRow>,
    active_list: DataTable<ModRow>,
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
    pub fn new(
        logger: Logger,
        mods: Arc<Mods>,
        on_action: impl Handler<ModManagerAction> + 'static,
    ) -> Rc<Self> {
        let mut grid = GridBuilder::with_factory(Tile::default_fill(), wrapper_factory());
        grid.row().with_stretch(1).add();

        let mut tile_limits = Group::default_fill();
        tile_limits.end();
        tile_limits.hide();

        grid.col().with_stretch(1).add();
        let mut available_list = DataTable::default().with_properties(DataTableProperties {
            columns: vec![
                ("Available Mods", Align::Left).into(),
                ("Version", Align::Left).into(),
                ("Author", Align::Left).into(),
            ],
            cell_padding: 4,
            cell_selection_color: fltk::enums::Color::Free,
            header_font_color: fltk::enums::Color::Gray0,
            ..Default::default()
        });
        available_list.make_resizable(true);
        available_list.set_row_header(false);
        available_list.set_col_header(true);
        available_list.set_col_resize(true);
        available_list.end();
        grid.cell()
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(SimpleWrapper::new(
                available_list.as_base_widget(),
                Default::default(),
            ));

        grid.col().add();

        let mut button_grid = Grid::builder_with_factory(wrapper_factory())
            .with_padding(10, 0, 10, 0)
            .with_col_spacing(10)
            .with_row_spacing(10);
        button_grid.col().add();

        button_grid.row().with_stretch(1).add();
        button_grid.cell().unwrap().skip();

        button_grid.row().add();
        let mut clear_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@filenew")
            .with_tooltip("Clear the mod list");
        button_grid.row().add();
        let mut import_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@fileopen")
            .with_tooltip("Import the mod list from a file");
        button_grid.row().add();
        let mut export_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@filesave")
            .with_tooltip("Export the mod list into a file");
        button_grid.row().add();
        let mut activate_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@>")
            .with_tooltip("Activate the selected mod");
        button_grid.row().add();
        let mut deactivate_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@<")
            .with_tooltip("Deactivate the selected mod");
        button_grid.row().add();
        let mut move_top_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@#8>|")
            .with_tooltip("Move the selected mod to top");
        button_grid.row().add();
        let mut move_up_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@#8>")
            .with_tooltip("Move the selected mod up");
        button_grid.row().add();
        let mut move_down_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@#2>")
            .with_tooltip("Move the selected mod down");
        button_grid.row().add();
        let mut move_bottom_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@#2>|")
            .with_tooltip("Move the selected mod to the bottom");
        button_grid.row().add();
        let mut more_info_button = button_grid
            .cell()
            .unwrap()
            .wrap(MenuButton::default())
            .with_label("\u{1f4dc}")
            .with_tooltip("Show information about the selected mod");
        more_info_button.deactivate();

        button_grid.row().with_stretch(1).add();
        button_grid.cell().unwrap().skip();

        let button_grid = button_grid.end();
        let mut button_col = button_grid.group();
        button_col.set_frame(FrameType::FlatBox);
        button_col.make_resizable(false);

        grid.cell()
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(button_grid);

        grid.col().with_stretch(1).add();
        let mut active_list = DataTable::default().with_properties(DataTableProperties {
            columns: vec![
                ("Active Mods", Align::Left).into(),
                ("Version", Align::Left).into(),
                ("Author", Align::Left).into(),
            ],
            cell_padding: 4,
            cell_selection_color: fltk::enums::Color::Free,
            header_font_color: fltk::enums::Color::Gray0,
            ..Default::default()
        });
        active_list.make_resizable(true);
        active_list.set_row_header(false);
        active_list.set_col_header(true);
        active_list.set_col_resize(true);
        active_list.end();
        grid.cell()
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(SimpleWrapper::new(
                active_list.as_base_widget(),
                Default::default(),
            ));

        let grid = grid.end();
        grid.layout_children(); // necessary for Tile

        adjust_col_widths(&mut available_list);
        adjust_col_widths(&mut active_list);

        let mut tiles = grid.group();
        tile_limits.resize(
            tiles.x() + button_col.width() * 2,
            tiles.y(),
            tiles.width() - button_col.width() * 4,
            tiles.height(),
        );
        tiles.resizable(&tile_limits);
        tiles.hide();

        let left_tile = available_list.as_base_widget();
        let mut mid_tile = button_col;
        let right_tile = active_list.as_base_widget();

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
            logger,
            tiles,
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
            state: RefCell::new(ModListState::new(mods)),
        });

        manager.update_actions();

        available_list.set_callback(manager.weak_cb(|this| {
            if is_table_nav_event() && this.available_list.callback_context() == TableContext::Cell
            {
                this.available_clicked();
            }
        }));

        active_list.set_callback(manager.weak_cb(|this| {
            if is_table_nav_event() && this.active_list.callback_context() == TableContext::Cell {
                this.active_clicked();
            }
        }));

        clear_button.set_callback(manager.weak_cb(Self::clear_clicked));
        import_button.set_callback(manager.weak_cb(Self::import_clicked));
        export_button.set_callback(manager.weak_cb(Self::export_clicked));
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
        let mut tiles = self.tiles.clone();
        tiles.show();

        if let Err(err) = (self.on_action)(ModManagerAction::LoadModList) {
            error!(self.logger, "Error loading mod list"; "error" => %err);
            alert_error(ERR_LOADING_MOD_LIST, &err);
        }

        Box::new(move || {
            tiles.hide();
        })
    }

    pub fn handle_update(&self, update: ModManagerUpdate) {
        match update {
            ModManagerUpdate::PopulateModList(active_mods) => self.populate_state(active_mods),
        }
    }

    declare_weak_cb!();

    fn populate_state(&self, active_mods: Vec<ModRef>) {
        let mut state = self.state.borrow_mut();
        let mod_count = state.installed.len();

        state.available = Vec::with_capacity(mod_count);
        state.active = Vec::with_capacity(mod_count);

        let mut available_set = BitVec::from_elem(mod_count, true);
        for mod_ref in active_mods {
            if let ModRef::Installed(mod_idx) = mod_ref {
                available_set.set(mod_idx, false);
            }
            state.active.push(mod_ref);
        }

        for mod_idx in 0..mod_count {
            if available_set[mod_idx] {
                state.available.push(ModRef::Installed(mod_idx));
            }
        }

        drop(state);

        self.populate_tables();
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

    fn clear_clicked(&self) {
        if self.state.borrow().active.is_empty() || !prompt_confirm(PROMPT_CLEAR_MODS) {
            return;
        }
        if self.save_mod_list(Vec::new()) {
            self.populate_state(Vec::new());
        }
    }

    fn import_clicked(&self) {
        if let Err(err) = (self.on_action)(ModManagerAction::ImportModList) {
            error!(self.logger, "Error importing mod list"; "error" => %err);
            alert_error(ERR_LOADING_MOD_LIST, &err);
        }
    }

    fn export_clicked(&self) {
        let state = self.state.borrow();
        if let Err(err) = (self.on_action)(ModManagerAction::ExportModList(state.active.clone())) {
            error!(self.logger, "Error exporting mod list"; "error" => %err);
            alert_error(ERR_SAVING_MOD_LIST, &err);
        }
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
        self.save_current_mod_list();
    }

    fn deactivate_clicked(&self) {
        let mut state = self.state.borrow_mut();
        let row_idx = state.get_selected_active().unwrap();

        let mod_ref = state.active.remove(row_idx);
        if let ModRef::Installed(mod_idx) = &mod_ref {
            let dest_row_idx = state
                .available
                .binary_search_by_key(mod_idx, |mod_ref| mod_ref.to_index().unwrap())
                .unwrap_err();
            state.available.insert(dest_row_idx, mod_ref);

            let row = mutate_table(&mut self.active_list.clone(), |data| data.remove(row_idx));
            mutate_table(&mut self.available_list.clone(), |data| {
                data.insert(dest_row_idx, row)
            });
        }

        drop(state);

        self.set_selection(None);
        self.save_current_mod_list();
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
        self.save_current_mod_list();
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
        self.save_current_mod_list();
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
        self.save_current_mod_list();
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
        self.save_current_mod_list();
    }

    fn save_current_mod_list(&self) {
        let state = self.state.borrow();
        self.save_mod_list(state.active.clone());
    }

    fn save_mod_list(&self, mod_list: Vec<ModRef>) -> bool {
        match (self.on_action)(ModManagerAction::SaveModList(mod_list)) {
            Ok(()) => true,
            Err(err) => {
                error!(self.logger, "Error saving mod list"; "error" => %err);
                alert_error(ERR_SAVING_MOD_LIST, &err);
                false
            }
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

        // TODO: Ensure main loop is run
        while popup.shown() {
            app::wait();
        }
    }
}

const PROMPT_CLEAR_MODS: &str = "Are you sure you want to clear the mod list?";
const ERR_LOADING_MOD_LIST: &str = "Error while loading the mod list.";
const ERR_SAVING_MOD_LIST: &str = "Error while saving the mod list.";
const CSS_INFO_BODY: &str = include_str!("mod_info.css");

lazy_static! {
    static ref BBCODE: BBCode = BBCode::from_config(BBCodeTagConfig::extended(), None).unwrap();
}

fn adjust_col_widths(table: &mut DataTable<ModRow>) {
    let scrollbar_width = table.scrollbar_size();
    let scrollbar_width =
        if scrollbar_width > 0 { scrollbar_width } else { fltk::app::scrollbar_size() };

    let width = table.width() - table.col_width(1) - table.col_width(2) - scrollbar_width - 2;
    table.set_col_width(0, width);
}

fn populate_table(table: &DataTable<ModRow>, mods: &Mods, refs: &Vec<ModRef>) {
    let rows = table.data();
    let mut rows = rows.borrow_mut();
    rows.clear();

    for mod_ref in refs {
        rows.push(make_mod_row(&mods, mod_ref));
    }
    drop(rows);

    table.updated(DataTableUpdate::DATA);
}

fn make_mod_row(mods: &Mods, mod_ref: &ModRef) -> ModRow {
    if let Some(mod_info) = mods.get(mod_ref) {
        let version = mod_info.version.to_string();
        let version =
            if mod_info.needs_update() { format!("@reload {}", version) } else { version };
        [mod_info.name.clone(), version, mod_info.author.clone()]
    } else {
        [
            match mod_ref {
                ModRef::Installed(_) => unreachable!(),
                ModRef::UnknownFolder(folder) => format!("??? ({})", folder),
                ModRef::UnknownPakPath(path) => format!("??? ({})", path.display()),
            },
            "???".to_string(),
            "???".to_string(),
        ]
    }
}

fn mutate_table<R>(table: &DataTable<ModRow>, mutator: impl FnOnce(&mut Vec<ModRow>) -> R) -> R {
    let data = table.data();
    let mut data = data.borrow_mut();
    let result = mutator(&mut data);
    drop(data);
    table.updated(DataTableUpdate::DATA);
    result
}
