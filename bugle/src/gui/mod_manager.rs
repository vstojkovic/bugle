use std::borrow::Cow;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use bbscope::{BBCode, BBCodeTagConfig};
use bit_vec::BitVec;
use fltk::app;
use fltk::button::Button;
use fltk::dialog::{alert_default, FileDialogOptions, FileDialogType, NativeFileChooser};
use fltk::enums::{Align, Event, FrameType};
use fltk::group::{Group, Tile};
use fltk::prelude::*;
use fltk::table::TableContext;
use fltk::window::Window;
use fltk_float::grid::{CellAlign, Grid, GridBuilder};
use fltk_float::{EmptyElement, LayoutElement, SimpleWrapper};
use fltk_webview::Webview;
use lazy_static::lazy_static;
use size::Size;
use slog::{error, Logger};

use crate::game::{Game, ModEntry, ModProvenance, ModRef, Mods};
use crate::mod_manager::ModManager;
use crate::util::weak_cb;

use super::prelude::*;
use super::widgets::{
    use_inspector_macros, DataTable, DataTableProperties, DataTableUpdate, Inspector,
    PropertiesTable, PropertyRow,
};
use super::{alert_error, is_table_nav_event, prompt_confirm, wrapper_factory};

enum Selection {
    Available(usize),
    Active(usize),
}

impl Selection {
    fn from_row(ctor: fn(usize) -> Self, row_idx: i32) -> Option<Self> {
        if row_idx >= 0 {
            Some(ctor(row_idx as _))
        } else {
            None
        }
    }
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

    fn selected_mod(&self) -> Option<&ModEntry> {
        match self.selection {
            None => None,
            Some(Selection::Available(idx)) => self.installed.get(&self.available[idx]),
            Some(Selection::Active(idx)) => self.installed.get(&self.active[idx]),
        }
    }
}

type ModRow = [String; 4];

pub(super) struct ModManagerTab {
    logger: Logger,
    game: Arc<Game>,
    mod_mgr: Rc<ModManager>,
    grid: Grid<Tile>,
    root: Tile,
    available_list: DataTable<ModRow>,
    active_list: DataTable<ModRow>,
    details_table: PropertiesTable<ModEntry, ()>,
    fix_errors_button: Button,
    activate_button: Button,
    deactivate_button: Button,
    move_top_button: Button,
    move_up_button: Button,
    move_down_button: Button,
    move_bottom_button: Button,
    description_button: Button,
    change_notes_button: Button,
    update_mods_button: Button,
    state: RefCell<ModListState>,
}

impl ModManagerTab {
    pub fn new(logger: &Logger, game: Arc<Game>, mod_mgr: Rc<ModManager>) -> Rc<Self> {
        let mut row_tiles = GridBuilder::with_factory(Tile::default_fill(), wrapper_factory());
        row_tiles.col().with_stretch(1).add();

        let mut row_tile_limits = Group::default_fill();
        row_tile_limits.end();
        row_tile_limits.hide();

        let mut col_tiles = GridBuilder::with_factory(Tile::default_fill(), wrapper_factory());
        col_tiles.row().with_stretch(1).add();

        let mut col_tile_limits = Group::default_fill();
        col_tile_limits.end();
        col_tile_limits.hide();

        col_tiles.col().with_stretch(1).add();
        let mut available_list = DataTable::default().with_properties(DataTableProperties {
            columns: vec![
                ("", 24).into(),
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
        col_tiles
            .cell()
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(SimpleWrapper::new(
                available_list.as_base_widget(),
                Default::default(),
            ));

        col_tiles.col().add();

        let mut button_grid = Grid::builder_with_factory(wrapper_factory())
            .with_padding(10, 0, 10, 0)
            .with_col_spacing(10)
            .with_row_spacing(6);
        button_grid.col().add();

        button_grid.row().with_stretch(1).add();
        button_grid.cell().unwrap().skip();

        button_grid.row().add();
        let mut clear_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@file_clear")
            .with_tooltip("Clear the mod list");
        button_grid.row().add();
        let mut import_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@folder_open")
            .with_tooltip("Import the mod list from a file");
        button_grid.row().add();
        let mut export_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@floppy_line")
            .with_tooltip("Export the mod list into a file");
        button_grid.row().add();
        let mut copy_modlist_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@clipboard_data")
            .with_tooltip("Copy the server launcher mod list to clipboard");
        button_grid.row().add();
        let mut fix_errors_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@fix_errors")
            .with_tooltip("Try to fix the errors in the mod list");
        fix_errors_button.deactivate();
        button_grid.row().add();
        button_grid
            .cell()
            .unwrap()
            .with_top_padding(6)
            .add(EmptyElement);
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
        button_grid
            .cell()
            .unwrap()
            .with_top_padding(6)
            .add(EmptyElement);
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
        button_grid
            .cell()
            .unwrap()
            .with_top_padding(6)
            .add(EmptyElement);
        button_grid.row().add();
        let mut description_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@info")
            .with_tooltip("Show selected mod's description");
        description_button.deactivate();
        button_grid.row().add();
        let mut change_notes_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@news")
            .with_tooltip("Show selected mod's change notes");
        change_notes_button.deactivate();
        button_grid.row().add();
        button_grid
            .cell()
            .unwrap()
            .with_top_padding(6)
            .add(EmptyElement);
        button_grid.row().add();
        let mut update_mods_button = button_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("@cloud_download")
            .with_tooltip("Update outdated mods");
        update_mods_button.deactivate();

        button_grid.row().with_stretch(1).add();
        button_grid.cell().unwrap().skip();

        let button_grid = Rc::new(button_grid.end());
        let mut button_col = button_grid.group();
        button_col.set_frame(FrameType::FlatBox);
        button_col.make_resizable(false);

        col_tiles
            .cell()
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add_shared(Rc::<Grid>::clone(&button_grid));

        col_tiles.col().with_stretch(1).add();
        let mut active_list = DataTable::default().with_properties(DataTableProperties {
            columns: vec![
                ("", 24).into(),
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
        col_tiles
            .cell()
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(SimpleWrapper::new(
                active_list.as_base_widget(),
                Default::default(),
            ));

        let col_tiles = col_tiles.end();
        col_tiles.layout_children(); // necessary for Tile
        let col_tiles_widget = col_tiles.group();

        row_tiles.row().with_stretch(4).add();
        row_tiles
            .cell()
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(col_tiles);

        available_list.set_flex_col(1);
        active_list.set_flex_col(1);

        col_tile_limits.resize(
            col_tiles_widget.x() + button_col.width() * 2,
            col_tiles_widget.y(),
            col_tiles_widget.width() - button_col.width() * 4,
            col_tiles_widget.height(),
        );
        col_tiles_widget.resizable(&col_tile_limits);

        let left_tile = available_list.as_base_widget();
        let mut mid_tile = button_col;
        let right_tile = active_list.as_base_widget();

        {
            let button_grid = Rc::clone(&button_grid);
            let fixed_width = mid_tile.width();
            let tiles = col_tiles_widget.clone();
            let mut left_tile = left_tile.clone();
            let mut right_tile = right_tile.clone();
            let mut old_x = mid_tile.x();
            mid_tile.resize_callback(move |tile, mut x, y, w, h| {
                if w == fixed_width {
                    button_grid.layout_children();
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

        row_tiles.row().with_stretch(1).add();
        let details_table = PropertiesTable::new((), MOD_DETAILS_ROWS, "Mod Details");
        row_tiles
            .cell()
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(SimpleWrapper::new(
                details_table.as_base_widget(),
                Default::default(),
            ));

        let grid = row_tiles.end();
        grid.layout_children();
        let mut root = grid.group();

        row_tile_limits.resize(
            root.x(),
            root.y() + button_grid.min_size().height,
            root.width(),
            root.height() - button_grid.min_size().height,
        );
        root.resizable(&row_tile_limits);

        root.hide();

        let state = RefCell::new(ModListState::new(Arc::clone(game.installed_mods())));

        let this = Rc::new(Self {
            logger: logger.clone(),
            game,
            mod_mgr,
            grid,
            root: root.clone(),
            available_list: available_list.clone(),
            active_list: active_list.clone(),
            details_table,
            fix_errors_button: fix_errors_button.clone(),
            activate_button: activate_button.clone(),
            deactivate_button: deactivate_button.clone(),
            move_top_button: move_top_button.clone(),
            move_up_button: move_up_button.clone(),
            move_down_button: move_down_button.clone(),
            move_bottom_button: move_bottom_button.clone(),
            description_button: description_button.clone(),
            change_notes_button: change_notes_button.clone(),
            update_mods_button: update_mods_button.clone(),
            state,
        });

        this.update_actions();

        root.handle(weak_cb!([this] => |_, event| {
            if let Event::Show = event {
                this.on_show();
            }
        }; false));

        available_list.set_callback(weak_cb!(
            [this] => |_| {
                if is_table_nav_event()
                    && this.available_list.callback_context() == TableContext::Cell
                {
                    if app::event_clicks() {
                        this.activate_clicked();
                    } else {
                        this.available_clicked();
                    }
                }
            }
        ));

        active_list.set_callback(weak_cb!(
            [this] => |_| {
                if is_table_nav_event() && this.active_list.callback_context() == TableContext::Cell
                {
                    if app::event_clicks() {
                        this.deactivate_clicked();
                    } else {
                        this.active_clicked();
                    }
                }
            }
        ));

        clear_button.set_callback(weak_cb!([this] => |_| this.clear_clicked()));
        import_button.set_callback(weak_cb!([this] => |_| this.import_clicked()));
        export_button.set_callback(weak_cb!([this] => |_| this.export_clicked()));
        copy_modlist_button.set_callback(weak_cb!([this] => |_| this.copy_modlist_clicked()));
        fix_errors_button.set_callback(weak_cb!([this] => |_| this.fix_errors_clicked()));
        activate_button.set_callback(weak_cb!([this] => |_| this.activate_clicked()));
        deactivate_button.set_callback(weak_cb!([this] => |_| this.deactivate_clicked()));
        move_top_button.set_callback(weak_cb!([this] => |_| this.move_top_clicked()));
        move_up_button.set_callback(weak_cb!([this] => |_| this.move_up_clicked()));
        move_down_button.set_callback(weak_cb!([this] => |_| this.move_down_clicked()));
        move_bottom_button.set_callback(weak_cb!([this] => |_| this.move_bottom_clicked()));
        update_mods_button.set_callback(weak_cb!([this] => |_| this.update_mods_clicked()));
        description_button.set_callback(weak_cb!([this] => |_| this.show_description()));
        change_notes_button.set_callback(weak_cb!([this] => |_| this.show_change_notes()));

        this
    }

    pub fn root(&self) -> &impl WidgetExt {
        &self.root
    }

    fn on_show(&self) {
        self.mod_mgr.check_mod_updates();
        let active_mods = match self.game.load_mod_list() {
            Ok(mods) => mods,
            Err(err) => {
                error!(self.logger, "Error loading mod list"; "error" => %err);
                alert_error(ERR_LOADING_MOD_LIST, &err);
                return;
            }
        };
        self.populate_state(active_mods);
    }

    fn populate_state(&self, active_mods: Vec<ModRef>) {
        let mut state = self.state.borrow_mut();
        let mod_count = state.installed.len();

        state.available = Vec::with_capacity(mod_count);
        state.active = Vec::with_capacity(mod_count);

        let mut available_set = BitVec::from_elem(mod_count, true);
        let mut errors_found = false;
        for mod_ref in active_mods {
            if let ModRef::Installed(mod_idx) = mod_ref {
                available_set.set(mod_idx, false);
            }
            if let ModRef::UnknownPakPath(_) = mod_ref {
                errors_found = true;
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

        self.fix_errors_button.clone().set_activated(errors_found);
    }

    fn populate_tables(&self) {
        let state = self.state.borrow();
        self.update_mods_button
            .clone()
            .set_activated(state.installed.iter().any(|entry| entry.needs_update()));

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

        let selection = Selection::from_row(Selection::Available, table.callback_row());
        self.set_selection(selection);
    }

    fn active_clicked(&self) {
        let mut table = self.active_list.clone();
        let _ = table.take_focus();

        let selection = Selection::from_row(Selection::Active, table.callback_row());
        self.set_selection(selection);
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
        self.details_table.populate(state.selected_mod());
        drop(state);
        self.update_actions();
    }

    fn update_actions(&self) {
        let state = self.state.borrow();
        let (activate, deactivate, move_up, move_down) = match state.selection {
            None => (false, false, false, false),
            Some(Selection::Available(_)) => (true, false, false, false),
            Some(Selection::Active(idx)) => {
                let last_idx = state.active.len() - 1;
                (false, true, idx > 0, idx < last_idx)
            }
        };

        let more_info = state
            .selected_mod()
            .and_then(|entry| entry.info.as_ref().ok())
            .is_some();

        self.activate_button.clone().set_activated(activate);
        self.deactivate_button.clone().set_activated(deactivate);
        self.move_top_button.clone().set_activated(move_up);
        self.move_up_button.clone().set_activated(move_up);
        self.move_down_button.clone().set_activated(move_down);
        self.move_bottom_button.clone().set_activated(move_down);
        self.description_button.clone().set_activated(more_info);
        self.change_notes_button.clone().set_activated(more_info);
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
        let mut dialog = NativeFileChooser::new(FileDialogType::BrowseFile);
        dialog.set_filter(DLG_FILTER_MODLIST);
        dialog.set_directory(&mod_list_dir(&self.game)).ok();
        dialog.show();

        let mod_list_path = dialog.filename();
        if mod_list_path.as_os_str().is_empty() {
            return;
        }

        let active_mods = match self.mod_mgr.import_mod_list(&mod_list_path) {
            Ok(mods) => mods,
            Err(err) => {
                error!(self.logger, "Error importing mod list"; "error" => %err);
                alert_error(ERR_LOADING_MOD_LIST, &err);
                return;
            }
        };
        self.populate_state(active_mods);
    }

    fn export_clicked(&self) {
        let state = self.state.borrow();
        let mut dialog = NativeFileChooser::new(FileDialogType::BrowseSaveFile);
        dialog.set_filter(DLG_FILTER_MODLIST);
        dialog.set_directory(&mod_list_dir(&self.game)).ok();
        dialog.set_option(FileDialogOptions::SaveAsConfirm);
        dialog.show();

        let mut mod_list_path = dialog.filename();
        if mod_list_path.as_os_str().is_empty() {
            return;
        }
        if mod_list_path.extension().is_none() {
            mod_list_path.set_extension("txt");
        }

        if let Err(err) = self
            .game
            .save_mod_list_to(&mod_list_path, state.active.iter())
        {
            error!(self.logger, "Error exporting mod list"; "error" => %err);
            alert_error(ERR_SAVING_MOD_LIST, &err);
        }
    }

    fn copy_modlist_clicked(&self) {
        use std::fmt::Write;

        let state = self.state.borrow();
        let mut text = String::new();
        for mod_ref in state.active.iter() {
            if let Some(entry) = state.installed.get(mod_ref) {
                if let Some(id) = entry
                    .info
                    .as_ref()
                    .ok()
                    .and_then(|info| info.steam_file_id(self.game.branch()))
                {
                    write!(text, ",{}", id).unwrap();
                } else {
                    write!(text, ",{}", entry.pak_path.display()).unwrap();
                }
            } else {
                match mod_ref {
                    ModRef::Installed(_) => unreachable!(),
                    ModRef::Custom(_) => unreachable!(),
                    ModRef::UnknownPakPath(path) => write!(text, ",{}", path.display()).unwrap(),
                    ModRef::UnknownFolder(_) => (),
                }
            }
        }

        if text.is_empty() {
            fltk::app::copy("");
        } else {
            fltk::app::copy(&text[1..]);
        }
    }

    fn fix_errors_clicked(&self) {
        let mut mod_list = {
            let state = self.state.borrow();
            state.active.clone()
        };
        if !self.mod_mgr.fix_mod_list(&mut mod_list) {
            alert_default("Could not fix all of the errors in the mod list.");
        }
        if let Err(err) = self.game.save_mod_list(mod_list.iter()) {
            error!(self.logger, "Error saving mod list"; "error" => %err);
            alert_error(ERR_SAVING_MOD_LIST, &err);
            return;
        }
        self.populate_state(mod_list);
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
        let row = mutate_table(&mut self.active_list.clone(), |data| data.remove(row_idx));
        if let ModRef::Installed(mod_idx) = &mod_ref {
            let dest_row_idx = state
                .available
                .binary_search_by_key(mod_idx, |mod_ref| mod_ref.to_index().unwrap())
                .unwrap_err();
            state.available.insert(dest_row_idx, mod_ref);

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

    fn update_mods_clicked(&self) {
        let outdated_mods = self
            .game
            .installed_mods()
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.needs_update())
            .map(|(idx, _)| ModRef::Installed(idx))
            .collect();
        self.mod_mgr.update_mods(outdated_mods);
        self.mod_mgr.check_mod_updates();
        self.populate_tables();
    }

    fn save_current_mod_list(&self) {
        let state = self.state.borrow();
        self.save_mod_list(state.active.clone());
    }

    fn save_mod_list(&self, mod_list: Vec<ModRef>) -> bool {
        match self.game.save_mod_list(mod_list.iter()) {
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
        let info = state.selected_mod().unwrap().info.as_ref().unwrap();
        self.show_bbcode(&format!("Description: {}", &info.name), &info.description);
    }

    fn show_change_notes(&self) {
        let state = self.state.borrow();
        let info = state.selected_mod().unwrap().info.as_ref().unwrap();
        self.show_bbcode(&format!("Change Notes: {}", &info.name), &info.change_notes);
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

        while popup.shown() && !app::should_program_quit() {
            app::wait();
        }
    }
}

impl LayoutElement for ModManagerTab {
    fn min_size(&self) -> fltk_float::Size {
        self.grid.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.grid.layout(x, y, width, height);
    }
}

const DLG_FILTER_MODLIST: &str = "Mod List Files\t*.txt";
const PROMPT_CLEAR_MODS: &str = "Are you sure you want to clear the mod list?";
const ERR_LOADING_MOD_LIST: &str = "Error while loading the mod list.";
const ERR_SAVING_MOD_LIST: &str = "Error while saving the mod list.";
const CSS_INFO_BODY: &str = include_str!("mod_info.css");

use_inspector_macros!(ModEntry, ());
macro_rules! info_attr {
    ($lambda:expr) => {
        |entry| {
            entry
                .info
                .as_ref()
                .map($lambda)
                .ok()
                .unwrap_or("???".into())
        }
    };
}

const MOD_DETAILS_ROWS: &[Inspector<ModEntry, ()>] = &[
    inspect_opt_attr!("Problem", |entry| entry
        .info
        .as_ref()
        .err()
        .map(|err| err.to_string().into())),
    inspect_attr!("Filename", |entry| entry
        .pak_path
        .display()
        .to_string()
        .into()),
    inspect_attr!("Size", |entry| format!(
        "{}",
        Size::from_bytes(entry.pak_size)
            .format()
            .with_base(size::Base::Base10)
    )
    .into()),
    inspect_attr!("Name", info_attr!(|info| info.name.clone().into())),
    inspect_author,
    inspect_attr!(
        "Version",
        info_attr!(|info| info.version.to_string().into())
    ),
    inspect_attr!(
        "Devkit Version",
        info_attr!(|info| format!("{}/{}", info.devkit_revision, info.devkit_snapshot).into())
    ),
    inspect_opt_attr!("Steam ID (Live)", |entry| entry
        .info
        .as_ref()
        .ok()
        .and_then(|info| opt_str_value(&info.live_steam_file_id))),
    inspect_opt_attr!("Steam ID (TestLive)", |entry| entry
        .info
        .as_ref()
        .ok()
        .and_then(|info| opt_str_value(&info.testlive_steam_file_id))),
];

lazy_static! {
    static ref BBCODE: BBCode = BBCode::from_config(BBCodeTagConfig::extended(), None).unwrap();
}

fn mod_list_dir(game: &Arc<Game>) -> &Path {
    let path = game.save_path();
    std::fs::create_dir_all(path).ok();
    path
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
    if let Some(entry) = mods.get(mod_ref) {
        if let Ok(info) = &entry.info {
            let version = info.version.to_string();
            let version =
                if entry.needs_update() { format!("@cloud_download {}", version) } else { version };
            [
                provenance_glyph(entry.provenance),
                info.name.clone(),
                version,
                info.author.clone(),
            ]
        } else {
            make_err_row(entry.pak_path.display())
        }
    } else {
        match mod_ref {
            ModRef::Installed(_) => unreachable!(),
            ModRef::Custom(_) => unreachable!(),
            ModRef::UnknownFolder(folder) => make_err_row(folder),
            ModRef::UnknownPakPath(path) => make_err_row(path.display()),
        }
    }
}

fn make_err_row<N: std::fmt::Display>(alt_name: N) -> ModRow {
    [
        "@error".to_string(),
        format!("??? ({})", alt_name),
        "???".to_string(),
        "???".to_string(),
    ]
}

fn provenance_glyph(provenance: ModProvenance) -> String {
    match provenance {
        ModProvenance::Local => "@folder".to_string(),
        ModProvenance::Steam => "@steam".to_string(),
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

fn inspect_author(
    _: &(),
    entry: Option<&ModEntry>,
    row_consumer: &mut dyn FnMut(PropertyRow),
    _include_empty: bool,
) {
    const HEADER: &str = "Author";

    let entry = match entry {
        Some(entry) => entry,
        None => {
            row_consumer([HEADER.into(), "".into()]);
            return;
        }
    };

    let info = match entry.info.as_ref() {
        Ok(info) => info,
        Err(_) => {
            row_consumer([HEADER.into(), "???".into()]);
            return;
        }
    };

    row_consumer([HEADER.into(), info.author.clone().into()]);
    if let Some(url) = opt_str_value(&info.author_url) {
        row_consumer(["".into(), url]);
    }
}

fn opt_str_value(value: &Option<String>) -> Option<Cow<'static, str>> {
    match value.as_ref() {
        None => None,
        Some(s) => {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string().into())
            }
        }
    }
}
