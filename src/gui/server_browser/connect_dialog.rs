use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use std::str::FromStr;

use fltk::button::{Button, ReturnButton};
use fltk::enums::Align;
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::input::{Input, SecretInput};
use fltk::prelude::*;
use fltk::window::Window;

use crate::gui::prelude::LayoutExt;
use crate::gui::{alert_error, button_row_height, widget_auto_height, widget_col_width};
use crate::servers::Server;

use super::ServerBrowserAction;

pub struct ConnectDialog {
    window: Window,
    result: Rc<RefCell<Option<ServerBrowserAction>>>,
}

impl ConnectDialog {
    pub fn direct_connect(parent: &Group) -> Self {
        let (window, mut server_text, password_text, mut ok_button) =
            Self::create_gui(parent, "Direct Connect", Input::default);

        let result = Rc::new(RefCell::new(None));

        server_text.set_value("127.0.0.1:7777");

        ok_button.set_callback({
            let server_text = server_text.clone();
            let password_text = password_text.clone();
            let result = Rc::clone(&result);
            let mut window = window.clone();
            move |_| {
                let addr = SocketAddr::from_str(&server_text.value()).map_err(anyhow::Error::msg);
                match addr {
                    Err(err) => alert_error(ERR_INVALID_ADDR, &err),
                    Ok(addr) => {
                        let password = password_text.value();
                        let password = if password.is_empty() { None } else { Some(password) };
                        *result.borrow_mut() = Some(ServerBrowserAction::JoinServer {
                            addr,
                            password,
                            battleye_required: None,
                        });
                        window.hide();
                    }
                }
            }
        });

        Self { window, result }
    }

    pub fn server_password(parent: &Group, server: &Server) -> Self {
        let (window, _, password_text, mut ok_button) =
            Self::create_gui(parent, "Enter Server Password", || {
                Frame::default()
                    .with_label(&server.name)
                    .with_align(Align::Left | Align::Inside)
            });

        let result = Rc::new(RefCell::new(None));

        ok_button.set_callback({
            let addr = server.game_addr().unwrap();
            let battleye_required = Some(server.battleye_required);
            let password_text = password_text.clone();
            let result = Rc::clone(&result);
            let mut window = window.clone();
            move |_| {
                let password = password_text.value();
                let password = if password.is_empty() { None } else { Some(password) };
                *result.borrow_mut() = Some(ServerBrowserAction::JoinServer {
                    addr,
                    password,
                    battleye_required,
                });
                window.hide();
            }
        });

        Self { window, result }
    }

    pub fn show(&self) {
        let mut window = self.window.clone();
        window.make_modal(true);
        window.show();
    }

    pub fn result(&self) -> Option<ServerBrowserAction> {
        self.result.borrow_mut().take()
    }

    pub fn shown(&self) -> bool {
        self.window.shown()
    }

    fn create_gui<T: WidgetExt>(
        parent: &Group,
        title: &'static str,
        make_server_text_widget: impl FnOnce() -> T,
    ) -> (Window, T, SecretInput, ReturnButton) {
        let mut window = Window::default().with_size(480, 135).with_label(title);

        let label_align = Align::Right | Align::Inside;
        let server_label = Frame::default()
            .with_label("Connect to:")
            .with_align(label_align);
        let server_text = make_server_text_widget();
        let password_label = Frame::default()
            .with_label("Password:")
            .with_align(label_align);
        let password_text = SecretInput::default();
        let ok_button = ReturnButton::default().with_label("OK");
        let cancel_button = Button::default().with_label("Cancel");

        let label_width = widget_col_width(&[&server_label, &password_label]);
        let text_width = window.width() - label_width - 30;
        let text_height = widget_auto_height(&server_label);
        let button_width = widget_col_width(&[&ok_button, &cancel_button]);
        let button_height = button_row_height(&[&ok_button, &cancel_button]);

        let server_label = server_label
            .with_size(label_width, text_height)
            .inside_parent(10, 10);
        let server_text = server_text
            .with_size(text_width, text_height)
            .right_of(&server_label, 10);
        let password_label = password_label
            .with_size(label_width, text_height)
            .below_of(&server_label, 10);
        let password_text = password_text
            .with_size(text_width, text_height)
            .right_of(&password_label, 10);
        let mut cancel_button = cancel_button
            .with_size(button_width, button_height)
            .inside_parent(-(button_width + 10), -(button_height + 10));
        let ok_button = ok_button
            .with_size(button_width, button_height)
            .left_of(&cancel_button, 10);

        window.end();
        window.set_pos(
            parent.x() + (parent.w() - window.w()) / 2,
            parent.y() + (parent.h() - window.h()) / 2,
        );

        cancel_button.set_callback({
            let mut window = window.clone();
            move |_| window.hide()
        });

        (window, server_text, password_text, ok_button)
    }
}

const ERR_INVALID_ADDR: &str = "Invalid server address.";
