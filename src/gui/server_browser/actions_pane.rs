use std::rc::Rc;

use fltk::button::Button;
use fltk::group::Group;
use fltk::prelude::*;

use crate::gui::prelude::*;
use crate::gui::{button_row_height, widget_auto_width};

pub enum Action {
    DirectConnect,
    Refresh,
    Ping,
    Join,
}

pub(super) struct ActionsPane {
    root: Group,
    direct_conn_button: Button,
    refresh_button: Button,
    ping_button: Button,
    join_button: Button,
}

impl ActionsPane {
    pub fn new() -> Rc<Self> {
        let root = Group::default_fill();

        let direct_conn_button = Button::default().with_label("Direct Connect...");
        let refresh_button = Button::default().with_label("Refresh");
        let ping_button = Button::default().with_label("Ping");
        let join_button = Button::default().with_label("Join");
        let button_height = button_row_height(&[
            &direct_conn_button,
            &refresh_button,
            &ping_button,
            &join_button,
        ]);

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

        root.end();

        Rc::new(Self {
            root,
            direct_conn_button,
            refresh_button,
            ping_button,
            join_button,
        })
    }

    pub fn root(&self) -> &Group {
        &self.root
    }

    pub fn set_server_actions_enabled(&self, enabled: bool) {
        let mut ping_button = self.ping_button.clone();
        let mut join_button = self.join_button.clone();

        if enabled {
            ping_button.activate();
            join_button.activate();
        } else {
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
            let mut ping_button = self.ping_button.clone();
            let on_action = Rc::clone(&on_action);
            ping_button.set_callback(move |_| on_action(Action::Ping));
        }
        {
            let mut join_button = self.join_button.clone();
            let on_action = Rc::clone(&on_action);
            join_button.set_callback(move |_| on_action(Action::Join));
        }
    }
}
