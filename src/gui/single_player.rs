use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use fltk::app;
use fltk::button::Button;
use fltk::enums::{Align, CallbackTrigger, Event};
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::misc::InputChoice;
use fltk::prelude::*;
use fltk::table::TableContext;
use fltk_table::{SmartTable, TableOpts};

use crate::game::{GameDB, Maps};

use super::data::{IterableTableSource, RowComparator, RowFilter, RowOrder, TableView};
use super::prelude::{LayoutExt, WidgetConvenienceExt};
use super::{button_row_height, widget_col_width, CleanupFn, Handler};

pub enum SinglePlayerAction {
    ListSavedGames,
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
    root: Group,
    on_action: Box<dyn Handler<SinglePlayerAction>>,
    in_progress_table: SmartTable,
    backups_table: SmartTable,
    continue_button: Button,
    load_button: Button,
    save_button: Button,
    save_as_button: Button,
    delete_button: Button,
    maps: Arc<Maps>,
    state: RefCell<SinglePlayerState>,
}

impl SinglePlayer {
    pub fn new(maps: Arc<Maps>, on_action: impl Handler<SinglePlayerAction> + 'static) -> Rc<Self> {
        let mut root = Group::default_fill();

        let label_align = Align::Right | Align::Inside;

        let map_label = Frame::default().with_label("Map:").with_align(label_align);
        let in_progress_label = Frame::default()
            .with_label("In Progress:")
            .with_align(label_align);
        let backups_label = Frame::default()
            .with_label("Backups:")
            .with_align(label_align);

        let new_button = Button::default().with_label("New");
        let continue_button = Button::default().with_label("Continue");
        let load_button = Button::default().with_label("Load");
        let save_button = Button::default().with_label("Save");
        let save_as_button = Button::default().with_label("Save As...");
        let delete_button = Button::default().with_label("Delete");

        let label_width = widget_col_width(&[&map_label, &in_progress_label, &backups_label]);
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
        map_input.set_value_index(0);
        let selected_map_id = maps.iter().next().unwrap().id;

        let in_progress_label = in_progress_label
            .below_of(&map_label, 10)
            .with_size(label_width, row_height);
        let in_progress_pane = Group::default_fill()
            .below_of(&map_input, 10)
            .stretch_to_parent(0, 0)
            .with_size_flex(0, row_height * 2);
        let in_progress_table = make_db_list();
        in_progress_pane.end();

        let _backups_label = backups_label
            .with_pos(
                in_progress_label.x(),
                in_progress_pane.y() + in_progress_pane.h() + 10,
            )
            .with_size(label_width, row_height);
        let backups_pane = Group::default_fill()
            .below_of(&in_progress_pane, 10)
            .stretch_to_parent(0, 0);
        let mut backups_table = make_db_list();
        backups_pane.end();

        root.end();
        root.hide();

        let single_player = Rc::new(Self {
            root,
            on_action: Box::new(on_action),
            in_progress_table,
            backups_table: backups_table.clone(),
            continue_button,
            load_button,
            save_button,
            save_as_button,
            delete_button,
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
                    if let Event::Released = app::event() {
                        if app::event_is_click() {
                            this.backup_clicked();
                        }
                    }
                }
            });
        }

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
                Err(err) => super::alert_error(ERR_LISTING_SAVED_GAMES, &err),
            },
        }
    }

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

    fn populate_list(&self) {
        {
            self.state.borrow_mut().selected_backup_idx = None;
        }

        let state = self.state.borrow();

        let selected_map_id = state.filter().map_id;

        let mut in_progress_table = self.in_progress_table.clone();
        if let Some(in_progress) = state.in_progress.get(&selected_map_id) {
            let data_ref = in_progress_table.data_ref();
            data_ref.lock().unwrap()[0] = make_row(in_progress);
        } else {
            in_progress_table.clear();
        }
        in_progress_table.redraw();

        let mut backups_table = self.backups_table.clone();
        let row_count = {
            let data_ref = backups_table.data_ref();
            let mut rows = data_ref.lock().unwrap();
            rows.clear();
            for saved_game in state.backups.iter() {
                rows.push(make_row(saved_game));
            }
            rows.len() as i32
        };
        backups_table.set_rows(row_count);
        backups_table.redraw();

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

fn make_db_list() -> SmartTable {
    let mut db_list = SmartTable::default_fill().with_opts(TableOpts {
        rows: 1,
        cols: 5,
        ..Default::default()
    });

    db_list.make_resizable(true);
    db_list.set_col_resize(true);
    db_list.set_row_header(false);
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

    db_list
}

fn make_row(game_db: &GameDB) -> Vec<String> {
    let lpc = game_db.last_played_char.as_ref();
    vec![
        game_db.file_name.display().to_string(),
        lpc.map(|lpc| lpc.last_played_timestamp.format("%c").to_string()).unwrap_or_default(),
        lpc.map(|lpc| lpc.name.clone()).unwrap_or_default(),
        lpc.map(|lpc| format!("{}", lpc.level)).unwrap_or_default(),
        lpc.and_then(|lpc| lpc.clan.as_ref()).map(String::clone).unwrap_or_default(),
    ]
}

const ERR_LISTING_SAVED_GAMES: &str = "Error while enumerating saves games.";
