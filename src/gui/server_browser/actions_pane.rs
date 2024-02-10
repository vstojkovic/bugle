use std::rc::Rc;

use fltk::button::{Button, CheckButton};
use fltk::enums::CallbackTrigger;
use fltk::prelude::*;
use fltk_float::grid::{CellAlign, Grid};
use fltk_float::LayoutElement;

use crate::gui::prelude::*;
use crate::gui::wrapper_factory;
use crate::servers::Server;

pub enum Action {
    DirectConnect,
    Refresh,
    ToggleSaved,
    ToggleFavorite,
    Ping,
    Join,
    ScrollLock(bool),
}

pub(super) struct ActionsPane {
    grid: Grid,
    direct_conn_button: Button,
    refresh_button: Button,
    toggle_saved_button: Option<Button>,
    toggle_favorite_button: Button,
    ping_button: Button,
    join_button: Button,
    scroll_lock_check: CheckButton,
}

impl ActionsPane {
    pub fn new(scroll_lock: bool, can_save_servers: bool) -> Rc<Self> {
        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10);
        grid.row().add();

        grid.col().add();
        let direct_conn_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Direct Connect...")
            .with_tooltip("Specify the address and port of the server to connect to");

        grid.col().add();
        let refresh_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Refresh")
            .with_tooltip("Reload the server list");

        grid.col().with_stretch(1).add();
        let scroll_lock_check = grid
            .cell()
            .unwrap()
            .with_horz_align(CellAlign::Center)
            .wrap(CheckButton::default())
            .with_label("Scroll lock")
            .with_tooltip("Make sure the selected server is always visible in the list");
        scroll_lock_check.set_checked(scroll_lock);

        grid.col().add();
        let mut toggle_saved_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Unsave")
            .with_tooltip("Toggle whether the selected server is in your saved servers");
        toggle_saved_button.deactivate();
        grid.col().add();

        let mut toggle_favorite_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Unfavorite")
            .with_tooltip("Toggle whether the selected server is in your favorites");
        toggle_favorite_button.deactivate();

        grid.col().add();
        let mut ping_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Ping")
            .with_tooltip(
                "Get updated information about the selected server's ping, age, and number of \
            connected players",
            );
        ping_button.deactivate();

        grid.col().add();
        let mut join_button = grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Join")
            .with_tooltip("Connect to the selected server");
        join_button.deactivate();

        let grid = grid.end();

        {
            let mut first_show = true;
            let mut toggle_saved_button = toggle_saved_button.clone();
            let mut toggle_favorite_button = toggle_favorite_button.clone();
            grid.group().handle(move |_, event| {
                if first_show && event == fltk::enums::Event::Show {
                    toggle_saved_button.set_label("Save");
                    toggle_favorite_button.set_label("Favorite");
                    first_show = false;
                }
                false
            });
        }

        let toggle_saved_button = if can_save_servers { Some(toggle_saved_button) } else { None };

        Rc::new(Self {
            grid,
            direct_conn_button,
            refresh_button,
            toggle_saved_button,
            toggle_favorite_button,
            ping_button,
            join_button,
            scroll_lock_check,
        })
    }

    pub fn element(self: &Rc<Self>) -> ActionsPaneElement {
        ActionsPaneElement {
            pane: Rc::clone(self),
        }
    }

    pub fn server_selected(&self, server: Option<&Server>) {
        let toggle_saved_button = self.toggle_saved_button.clone();
        let mut toggle_favorite_button = self.toggle_favorite_button.clone();
        let mut ping_button = self.ping_button.clone();
        let mut join_button = self.join_button.clone();

        if let Some(server) = server {
            if let Some(mut button) = toggle_saved_button {
                button.activate();
                button.set_label(if server.is_saved() { "Unsave" } else { "Save" });
            }

            toggle_favorite_button.activate();
            toggle_favorite_button.set_label(if server.favorite {
                "Unfavorite"
            } else {
                "Favorite"
            });

            ping_button.set_activated(server.is_valid());
            join_button.set_activated(server.is_valid());
        } else {
            if let Some(mut button) = toggle_saved_button {
                button.set_label("Save");
                button.deactivate();
            }
            toggle_favorite_button.set_label("Favorite");
            toggle_favorite_button.deactivate();
            ping_button.deactivate();
            join_button.deactivate();
        }
    }

    pub fn set_on_action(&self, on_action: impl Fn(Action) + 'static) {
        let on_action = Rc::new(on_action);
        {
            let mut direct_conn_button = self.direct_conn_button.clone();
            let on_action = Rc::clone(&on_action);
            direct_conn_button.set_callback(move |_| on_action(Action::DirectConnect));
        }
        {
            let mut refresh_button = self.refresh_button.clone();
            let on_action = Rc::clone(&on_action);
            refresh_button.set_callback(move |_| on_action(Action::Refresh));
        }
        if let Some(button) = self.toggle_saved_button.as_ref() {
            let mut toggle_saved_button = button.clone();
            let on_action = Rc::clone(&on_action);
            toggle_saved_button.set_callback(move |_| on_action(Action::ToggleSaved));
        }
        {
            let mut toggle_favorite_button = self.toggle_favorite_button.clone();
            let on_action = Rc::clone(&on_action);
            toggle_favorite_button.set_callback(move |_| on_action(Action::ToggleFavorite));
        }
        {
            let mut ping_button = self.ping_button.clone();
            let on_action = Rc::clone(&on_action);
            ping_button.set_callback(move |_| on_action(Action::Ping));
        }
        {
            let mut join_button = self.join_button.clone();
            let on_action = Rc::clone(&on_action);
            join_button.set_callback(move |_| on_action(Action::Join));
        }
        {
            let mut scroll_lock_check = self.scroll_lock_check.clone();
            let on_action = Rc::clone(&on_action);
            scroll_lock_check.set_trigger(CallbackTrigger::Changed);
            scroll_lock_check
                .set_callback(move |check| on_action(Action::ScrollLock(check.is_checked())));
        }
    }
}

pub(super) struct ActionsPaneElement {
    pane: Rc<ActionsPane>,
}

impl LayoutElement for ActionsPaneElement {
    fn min_size(&self) -> fltk_float::Size {
        self.pane.grid.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.pane.grid.layout(x, y, width, height)
    }
}
