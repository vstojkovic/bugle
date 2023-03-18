use std::rc::Rc;

use fltk::button::{Button, CheckButton};
use fltk::enums::CallbackTrigger;
use fltk::group::Group;
use fltk::prelude::*;

use crate::gui::prelude::*;
use crate::gui::{button_row_height, widget_auto_height, widget_auto_width};
use crate::servers::Server;

pub enum Action {
    DirectConnect,
    Refresh,
    ToggleFavorite,
    Ping,
    Join,
    ScrollLock(bool),
}

pub(super) struct ActionsPane {
    root: Group,
    direct_conn_button: Button,
    refresh_button: Button,
    toggle_favorite_button: Button,
    ping_button: Button,
    join_button: Button,
    scroll_lock_check: CheckButton,
}

impl ActionsPane {
    pub fn new(scroll_lock: bool) -> Rc<Self> {
        let root = Group::default_fill();

        let direct_conn_button = Button::default().with_label("Direct Connect...");
        let refresh_button = Button::default().with_label("Refresh");
        let toggle_favorite_button = Button::default().with_label("Unfavorite");
        let ping_button = Button::default().with_label("Ping");
        let join_button = Button::default().with_label("Join");
        let button_height = button_row_height(&[
            &direct_conn_button,
            &refresh_button,
            &toggle_favorite_button,
            &ping_button,
            &join_button,
        ]);

        let scroll_lock_check = CheckButton::default().with_label("Scroll lock");
        let scroll_lock_width = widget_auto_width(&scroll_lock_check);
        let scroll_lock_height = widget_auto_height(&scroll_lock_check);

        let root = root
            .with_size_flex(0, button_height + 20)
            .inside_parent(0, -(button_height + 20));

        let direct_connect_width = widget_auto_width(&direct_conn_button);
        let direct_conn_button = direct_conn_button
            .with_size(direct_connect_width, button_height)
            .inside_parent(0, 10);
        let refresh_width = widget_auto_width(&refresh_button);
        let refresh_button = refresh_button
            .with_size(refresh_width, button_height)
            .right_of(&direct_conn_button, 10);
        let join_width = widget_auto_width(&join_button);
        let mut join_button = join_button
            .with_size(join_width, button_height)
            .inside_parent(-join_width, 10);
        join_button.deactivate();
        let ping_width = widget_auto_width(&ping_button);
        let mut ping_button = ping_button
            .with_size(ping_width, button_height)
            .left_of(&join_button, 10);
        ping_button.deactivate();
        let toggle_favorite_width = widget_auto_width(&toggle_favorite_button);
        let mut toggle_favorite_button = toggle_favorite_button
            .with_size(toggle_favorite_width, button_height)
            .left_of(&ping_button, 10);
        toggle_favorite_button.deactivate();
        toggle_favorite_button.set_label("Favorite");

        let scroll_lock_check = scroll_lock_check
            .with_size(scroll_lock_width, scroll_lock_height)
            .center_of_parent();
        scroll_lock_check.set_checked(scroll_lock);

        root.end();

        Rc::new(Self {
            root,
            direct_conn_button,
            refresh_button,
            toggle_favorite_button,
            ping_button,
            join_button,
            scroll_lock_check,
        })
    }

    pub fn root(&self) -> &Group {
        &self.root
    }

    pub fn server_selected(&self, server: Option<&Server>) {
        let mut toggle_favorite_button = self.toggle_favorite_button.clone();
        let mut ping_button = self.ping_button.clone();
        let mut join_button = self.join_button.clone();

        if let Some(server) = server {
            toggle_favorite_button.activate();
            toggle_favorite_button.set_label(if server.favorite {
                "Unfavorite"
            } else {
                "Favorite"
            });
            ping_button.set_activated(server.is_valid());
            join_button.set_activated(server.is_valid());
        } else {
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
