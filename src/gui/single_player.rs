use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Component, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use anyhow::{bail, Result};
use fltk::button::Button;
use fltk::dialog;
use fltk::enums::CallbackTrigger;
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::misc::InputChoice;
use fltk::prelude::*;
use fltk::table::TableContext;
use fltk_float::grid::{CellAlign, Grid};
use fltk_float::SimpleWrapper;
use slog::{error, Logger};

use crate::game::{GameDB, Maps};

use super::data::{IterableTableSource, Reindex, RowComparator, RowFilter, RowOrder, TableView};
use super::prelude::*;
use super::widgets::{DataTable, DataTableProperties, DataTableUpdate};
use super::{alert_error, is_table_nav_event, prompt_confirm, wrapper_factory};
use super::{CleanupFn, Handler};

pub enum SinglePlayerAction {
    ListSavedGames,
    NewSavedGame { map_id: usize },
    ContinueSavedGame { map_id: usize },
    LoadSavedGame { map_id: usize, backup_name: PathBuf },
    SaveGame { map_id: usize, backup_name: PathBuf },
    DeleteSavedGame { backup_name: PathBuf },
}

pub enum SinglePlayerUpdate {
    PopulateList(Result<Vec<GameDB>>),
}

struct SavedGameFilter {
    map_id: usize,
}

impl RowFilter<GameDB> for SavedGameFilter {
    fn matches(&self, item: &GameDB) -> bool {
        item.map_id == self.map_id
    }
}

struct SavedGameOrder;

impl RowOrder<GameDB> for SavedGameOrder {
    fn comparator(&self) -> RowComparator<GameDB> {
        Box::new(|lhs, rhs| lhs.file_name.cmp(&rhs.file_name))
    }
}

struct SinglePlayerState {
    in_progress: HashMap<usize, GameDB>,
    backups: TableView<Vec<GameDB>, SavedGameFilter, SavedGameOrder>,
    selected_backup_idx: Option<usize>,
}

impl SinglePlayerState {
    fn new(map_id: usize) -> Self {
        Self {
            in_progress: HashMap::new(),
            backups: TableView::new(vec![], SavedGameFilter { map_id }, SavedGameOrder),
            selected_backup_idx: None,
        }
    }

    fn filter(&self) -> &SavedGameFilter {
        self.backups.filter()
    }
}

pub struct SinglePlayer {
    logger: Logger,
    root: Group,
    on_action: Box<dyn Handler<SinglePlayerAction>>,
    in_progress_table: DataTable<Vec<String>>,
    backups_table: DataTable<Vec<String>>,
    continue_button: Button,
    load_button: Button,
    save_button: Button,
    save_as_button: Button,
    delete_button: Button,
    maps: Arc<Maps>,
    state: RefCell<SinglePlayerState>,
}

impl SinglePlayer {
    pub fn new(
        logger: Logger,
        maps: Arc<Maps>,
        on_action: impl Handler<SinglePlayerAction> + 'static,
    ) -> Rc<Self> {
        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10);
        grid.col().with_default_align(CellAlign::End).add();
        grid.col().with_stretch(1).add();
        let btn_group = grid.col_group().add();
        grid.extend_group(btn_group).batch(6);

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Map:");
        let mut map_input = grid.cell().unwrap().wrap(InputChoice::default_fill());
        for map in maps.iter() {
            map_input.add(&map.display_name);
        }
        map_input.input().set_readonly(true);
        map_input.input().clear_visible_focus();
        map_input.set_value_index(0);
        let selected_map_id = maps.iter().next().unwrap().id;
        let mut new_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("New")
            .with_tooltip("Start a new singleplayer game from scratch");
        let mut continue_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Continue")
            .with_tooltip("Continue the current singleplayer game");
        let mut load_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Load")
            .with_tooltip("Replace the current singleplayer game with the selected backup");
        let mut save_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Save")
            .with_tooltip("Replace the selected backup with the current singleplayer game");
        let mut save_as_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Save As...")
            .with_tooltip("Create a new backup of the current singleplayer game");
        let mut delete_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Delete")
            .with_tooltip("Delete the selected backup");

        grid.row().with_stretch(1).add();
        grid.cell()
            .unwrap()
            .with_vert_align(CellAlign::Start)
            .wrap(Frame::default())
            .with_label("In Progress:");
        let in_progress_table = make_db_list();
        grid.span(1, 7)
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(SimpleWrapper::new(
                in_progress_table.as_base_widget(),
                Default::default(),
            ));

        grid.row().with_stretch(9).add();
        grid.cell()
            .unwrap()
            .with_vert_align(CellAlign::Start)
            .wrap(Frame::default())
            .with_label("Backups:");
        let mut backups_table = make_db_list();
        grid.span(1, 7)
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(SimpleWrapper::new(
                backups_table.as_base_widget(),
                Default::default(),
            ));

        let grid = grid.end();
        grid.layout_children();

        let mut root = grid.group();
        root.hide();
        root.resize_callback(move |_, _, _, _, _| grid.layout_children());

        let single_player = Rc::new(Self {
            logger,
            root,
            on_action: Box::new(on_action),
            in_progress_table,
            backups_table: backups_table.clone(),
            continue_button: continue_button.clone(),
            load_button: load_button.clone(),
            save_button: save_button.clone(),
            save_as_button: save_as_button.clone(),
            delete_button: delete_button.clone(),
            maps,
            state: RefCell::new(SinglePlayerState::new(selected_map_id)),
        });

        {
            let this = Rc::downgrade(&single_player);
            map_input.set_trigger(CallbackTrigger::Changed);
            map_input.set_callback(move |input| {
                if let Some(this) = this.upgrade() {
                    this.map_selected(input.menu_button().value() as _);
                }
            });
        }

        {
            let this = Rc::downgrade(&single_player);
            backups_table.set_callback(move |_| {
                if let Some(this) = this.upgrade() {
                    if is_table_nav_event() {
                        this.backup_clicked();
                    }
                }
            });
        }

        new_button.set_callback(single_player.weak_cb(Self::new_clicked));
        continue_button.set_callback(single_player.weak_cb(Self::continue_clicked));
        load_button.set_callback(single_player.weak_cb(Self::load_clicked));
        save_button.set_callback(single_player.weak_cb(Self::save_clicked));
        save_as_button.set_callback(single_player.weak_cb(Self::save_as_clicked));
        delete_button.set_callback(single_player.weak_cb(Self::delete_clicked));

        single_player
    }

    pub fn show(&self) -> CleanupFn {
        let mut root = self.root.clone();
        root.show();

        (self.on_action)(SinglePlayerAction::ListSavedGames).unwrap();

        Box::new(move || {
            root.hide();
        })
    }

    pub fn handle_update(&self, update: SinglePlayerUpdate) {
        match update {
            SinglePlayerUpdate::PopulateList(result) => match result {
                Ok(games) => self.set_games(games),
                Err(err) => {
                    error!(self.logger, "Error listing saved games"; "error" => %err);
                    super::alert_error(ERR_LISTING_SAVED_GAMES, &err);
                }
            },
        }
    }

    declare_weak_cb!();

    fn set_games(&self, mut games: Vec<GameDB>) {
        {
            let mut state = self.state.borrow_mut();

            state.in_progress.clear();
            let mut idx = 0;
            while idx < games.len() {
                let game = &games[idx];
                if game.file_name == self.maps[game.map_id].db_name {
                    state.in_progress.insert(game.map_id, games.remove(idx));
                } else {
                    idx += 1;
                }
            }

            state
                .backups
                .update_source(|saved_games| *saved_games = games);
        }

        self.populate_list();
    }

    fn map_selected(&self, idx: usize) {
        {
            let mut state = self.state.borrow_mut();
            let map_id = self.maps[idx].id;
            state.backups.update_filter(|filter| filter.map_id = map_id);
        }

        self.populate_list();
    }

    fn backup_clicked(&self) {
        if let TableContext::Cell = self.backups_table.callback_context() {
            let _ = self.backups_table.clone().take_focus();

            let selected_idx = self.backups_table.callback_row() as _;
            {
                self.state.borrow_mut().selected_backup_idx = Some(selected_idx);
            }
            self.update_actions();
        }
    }

    fn new_clicked(&self) {
        let state = self.state.borrow();
        let map_id = state.filter().map_id;
        if state.in_progress.contains_key(&map_id) && !prompt_confirm(PROMPT_REPLACE_IN_PROGRESS) {
            return;
        }
        drop(state);

        if let Err(err) = (self.on_action)(SinglePlayerAction::NewSavedGame { map_id }) {
            error!(self.logger, "Error launching singleplayer game"; "error" => %err);
            alert_error(ERR_LAUNCHING_SP, &err);
            return;
        }

        {
            let mut state = self.state.borrow_mut();
            state.in_progress.remove(&map_id);
        }
        self.populate_list();
    }

    fn continue_clicked(&self) {
        let map_id = self.state.borrow().filter().map_id;
        if let Err(err) = (self.on_action)(SinglePlayerAction::ContinueSavedGame { map_id }) {
            error!(self.logger, "Error launching singleplayer game"; "error" => %err);
            alert_error(ERR_LAUNCHING_SP, &err);
        }
    }

    fn load_clicked(&self) {
        let state = self.state.borrow();
        let backup_idx = state.selected_backup_idx.unwrap();
        let map_id = state.filter().map_id;
        if state.in_progress.contains_key(&map_id) && !prompt_confirm(PROMPT_REPLACE_IN_PROGRESS) {
            return;
        }
        let backup_name = state.backups[backup_idx].file_name.clone();
        let action = SinglePlayerAction::LoadSavedGame {
            map_id,
            backup_name,
        };
        drop(state);

        if let Err(err) = (self.on_action)(action) {
            error!(self.logger, "Error loading singleplayer backup"; "error" => %err);
            alert_error(ERR_LOADING_GAME, &err);
            return;
        }

        {
            let mut state = self.state.borrow_mut();
            let in_progress_name = &self.maps[map_id].db_name;
            let new_in_progress = GameDB::copy_from(&state.backups[backup_idx], in_progress_name);
            state.in_progress.insert(map_id, new_in_progress);
        }
        self.populate_list();
    }

    fn save_clicked(&self) {
        if !prompt_confirm(PROMPT_REPLACE_BACKUP) {
            return;
        }

        let state = self.state.borrow();
        let backup_idx = state.selected_backup_idx.unwrap();
        let map_id = state.filter().map_id;
        let backup_name = state.backups[backup_idx].file_name.clone();
        let action = SinglePlayerAction::SaveGame {
            map_id,
            backup_name,
        };
        drop(state);

        if let Err(err) = (self.on_action)(action) {
            error!(self.logger, "Error saving singleplayer backup"; "error" => %err);
            alert_error(ERR_SAVING_GAME, &err);
            return;
        }

        {
            let mut state = self.state.borrow_mut();
            let unfiltered_idx = state.backups.to_source_index(backup_idx);
            let backup_name = &state.backups[backup_idx].file_name;
            let updated_backup = GameDB::copy_from(&state.in_progress[&map_id], &backup_name);
            state.backups.update(|games, _, _| {
                games[unfiltered_idx] = updated_backup;
                Reindex::Nothing
            });
        }
        self.populate_list();
    }

    fn save_as_clicked(&self) {
        let backup_name = if let Some(name) = dialog::input_default(PROMPT_BACKUP_NAME, "") {
            db_name_from(name)
        } else {
            return;
        };
        let backup_name = match backup_name {
            Ok(name) => name,
            Err(err) => {
                alert_error(ERR_INVALID_BACKUP_NAME, &err);
                return;
            }
        };

        let state = self.state.borrow();
        let map_id = state.filter().map_id;
        let existing_idx = state
            .backups
            .source()
            .iter()
            .enumerate()
            .find(|(_, game)| game.file_name == backup_name)
            .map(|(idx, _)| idx);

        if existing_idx.is_some() && !prompt_confirm(PROMPT_REPLACE_BACKUP) {
            return;
        }

        let action = SinglePlayerAction::SaveGame {
            map_id,
            backup_name: backup_name.clone(),
        };
        drop(state);

        if let Err(err) = (self.on_action)(action) {
            error!(self.logger, "Error saving singleplayer backup"; "error" => %err);
            alert_error(ERR_SAVING_GAME, &err);
            return;
        }

        {
            let mut state = self.state.borrow_mut();
            let backup = GameDB::copy_from(&state.in_progress[&map_id], &backup_name);
            if let Some(idx) = existing_idx {
                state.backups.update(|games, _, _| {
                    let old_map_id = games[idx].map_id;
                    games[idx] = backup;
                    Reindex::Nothing.filter_if(old_map_id != map_id)
                });
            } else {
                state.backups.update_source(|games| games.push(backup));
            }
        }
        self.populate_list();
    }

    fn delete_clicked(&self) {
        if !prompt_confirm(PROMPT_DELETE_BACKUP) {
            return;
        }

        let state = self.state.borrow();
        let backup_idx = state.selected_backup_idx.unwrap();
        let backup_name = state.backups[backup_idx].file_name.clone();
        let action = SinglePlayerAction::DeleteSavedGame { backup_name };
        drop(state);

        if let Err(err) = (self.on_action)(action) {
            error!(self.logger, "Error deleting singleplayer backup"; "error" => %err);
            alert_error(ERR_DELETING_GAME, &err);
            return;
        }

        {
            let mut state = self.state.borrow_mut();
            let unfiltered_idx = state.backups.to_source_index(backup_idx);
            state.backups.update_source(|games| {
                games.remove(unfiltered_idx);
            });
        }
        self.populate_list();
    }

    fn populate_list(&self) {
        {
            self.state.borrow_mut().selected_backup_idx = None;
        }

        let state = self.state.borrow();

        let selected_map_id = state.filter().map_id;

        {
            let data = self.in_progress_table.data();
            let mut data = data.borrow_mut();
            if let Some(in_progress) = state.in_progress.get(&selected_map_id) {
                let row = make_row(in_progress);
                if data.is_empty() {
                    data.push(row)
                } else {
                    data[0] = row;
                }
            } else {
                data.clear();
            }
        }
        self.in_progress_table.updated(DataTableUpdate::DATA);

        {
            let data = self.backups_table.data();
            let mut data = data.borrow_mut();
            data.clear();
            for saved_game in state.backups.iter() {
                data.push(make_row(saved_game));
            }
        };
        self.backups_table.updated(DataTableUpdate::DATA);

        self.update_actions();
    }

    fn update_actions(&self) {
        let state = self.state.borrow();
        let in_progress_exists = state.in_progress.contains_key(&state.filter().map_id);
        let backup_selected = state.selected_backup_idx.is_some();

        self.continue_button
            .clone()
            .set_activated(in_progress_exists);
        self.load_button.clone().set_activated(backup_selected);
        self.save_button
            .clone()
            .set_activated(in_progress_exists && backup_selected);
        self.save_as_button
            .clone()
            .set_activated(in_progress_exists);
        self.delete_button.clone().set_activated(backup_selected);
    }
}

const ERR_LISTING_SAVED_GAMES: &str = "Error while enumerating saves games.";
const ERR_LAUNCHING_SP: &str = "Error while trying to launch the single-player game.";
const ERR_LOADING_GAME: &str = "Error while loading a saved game.";
const ERR_SAVING_GAME: &str = "Error while saving the in-progress game.";
const ERR_DELETING_GAME: &str = "Error while deleting a saved game.";
const ERR_INVALID_BACKUP_NAME: &str =
    "Invalid backup name. Please use a non-empty filename without a path.";
const ERR_PREFIX_INVALID_NAME: &str = "Invalid filename";
const PROMPT_REPLACE_IN_PROGRESS: &str = "Are you sure you want to overwrite the in-progress game?";
const PROMPT_REPLACE_BACKUP: &str = "Are you sure you want to overwrite this backup?";
const PROMPT_BACKUP_NAME: &str = "Backup name:";
const PROMPT_DELETE_BACKUP: &str = "Are you sure you want to delete this backup?";

fn make_db_list() -> DataTable<Vec<String>> {
    let mut db_list = DataTable::default().with_properties(DataTableProperties {
        columns: vec![
            ("Filename", 440).into(),
            ("Last Played", 200).into(),
            ("Character", 160).into(),
            ("Level", 50).into(),
            ("Clan", 150).into(),
        ],
        cell_selection_color: fltk::enums::Color::Free,
        header_font_color: fltk::enums::Color::Gray0,
        ..Default::default()
    });

    db_list.make_resizable(true);
    db_list.set_col_resize(true);
    db_list.set_col_header(true);
    db_list.set_row_header(false);
    db_list.end();

    db_list
}

fn make_row(game_db: &GameDB) -> Vec<String> {
    let lpc = game_db.last_played_char.as_ref();
    vec![
        game_db.file_name.display().to_string(),
        lpc.map(|lpc| lpc.last_played_timestamp.format("%c").to_string())
            .unwrap_or_default(),
        lpc.map(|lpc| lpc.name.clone()).unwrap_or_default(),
        lpc.map(|lpc| format!("{}", lpc.level)).unwrap_or_default(),
        lpc.and_then(|lpc| lpc.clan.as_ref())
            .map(String::clone)
            .unwrap_or_default(),
    ]
}

fn db_name_from(s: String) -> Result<PathBuf> {
    let s = s.trim();
    if s.is_empty() {
        bail!("Filename was empty.");
    }

    let mut db_name = PathBuf::from(s.trim());
    if db_name.parent() != Some("".as_ref()) {
        bail!("{}: {}", ERR_PREFIX_INVALID_NAME, s);
    }
    if let Some(Component::Normal(_)) = db_name.components().next() {
        match db_name.extension() {
            None => {
                db_name.set_extension("db");
            }
            Some(ext) => {
                if ext != "db" {
                    let mut ext = ext.to_owned();
                    ext.push(".db");
                    db_name.set_extension(ext);
                }
            }
        }
        Ok(db_name)
    } else {
        bail!("{}: {}", ERR_PREFIX_INVALID_NAME, s);
    }
}
