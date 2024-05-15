use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use std::str::FromStr;

use fltk::button::{Button, CheckButton, ReturnButton};
use fltk::enums::Align;
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::input::{Input, SecretInput};
use fltk::prelude::*;
use fltk::window::Window;
use fltk_float::grid::{CellAlign, GridBuilder};
use fltk_float::LayoutElement;

use crate::gui::{alert_error, wrapper_factory};
use crate::launcher::ConnectionInfo;
use crate::servers::Server;

pub struct ConnectDialog {
    window: Window,
    result: Rc<RefCell<Option<ConnectDialogResult>>>,
}

pub struct ConnectDialogResult {
    pub connection: ConnectionInfo,
    pub save_password: bool,
}

impl ConnectDialog {
    pub fn direct_connect(parent: &Group) -> Self {
        let (window, mut server_text, password_text, _, mut ok_button) =
            Self::create_gui(parent, "Direct Connect", None, Input::default);

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
                        *result.borrow_mut() = Some(ConnectDialogResult {
                            connection: ConnectionInfo {
                                addr,
                                password,
                                battleye_required: None,
                            },
                            save_password: false,
                        });
                        window.hide();
                    }
                }
            }
        });

        Self { window, result }
    }

    pub fn server_password(parent: &Group, server: &Server, password: &str) -> Self {
        let (window, _, password_text, save_password_check, mut ok_button) =
            Self::create_gui(parent, "Enter Server Password", Some(password), || {
                Frame::default()
                    .with_label(&server.name)
                    .with_align(Align::Left | Align::Inside)
            });
        let save_password_check = save_password_check.unwrap();

        let result = Rc::new(RefCell::new(None));

        ok_button.set_callback({
            let addr = server.game_addr().unwrap();
            let battleye_required = Some(server.general.battleye_required);
            let password_text = password_text.clone();
            let result = Rc::clone(&result);
            let mut window = window.clone();
            move |_| {
                let password = password_text.value();
                let save_password = save_password_check.is_checked() && !password.is_empty();
                let password = if password.is_empty() { None } else { Some(password) };
                *result.borrow_mut() = Some(ConnectDialogResult {
                    connection: ConnectionInfo {
                        addr,
                        password,
                        battleye_required,
                    },
                    save_password,
                });
                window.hide();
            }
        });

        Self { window, result }
    }

    pub fn run(&self) -> Option<ConnectDialogResult> {
        let mut window = self.window.clone();
        window.make_modal(true);
        window.show();

        while window.shown() && !fltk::app::should_program_quit() {
            fltk::app::wait();
        }

        self.result.borrow_mut().take()
    }

    fn create_gui<T: WidgetExt + Clone + 'static>(
        parent: &Group,
        title: &'static str,
        password: Option<&str>,
        make_server_text_widget: impl FnOnce() -> T,
    ) -> (Window, T, SecretInput, Option<CheckButton>, ReturnButton) {
        let mut window = GridBuilder::with_factory(
            Window::default().with_size(480, 160).with_label(title),
            wrapper_factory(),
        )
        .with_col_spacing(10)
        .with_row_spacing(10)
        .with_padding(10, 10, 10, 10);
        window.col().with_default_align(CellAlign::End).add();
        window.col().with_stretch(1).add();
        let btn_group = window.col_group().add();
        window.extend_group(btn_group).batch(2);

        window.row().add();
        window
            .cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Connect to:");
        let server_text = window.span(1, 3).unwrap().wrap(make_server_text_widget());

        window.row().add();
        window
            .cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Password:");
        let mut password_text = window.span(1, 3).unwrap().wrap(SecretInput::default());

        let save_password_check = password.map(|password| {
            password_text.set_value(password);

            window.row().add();
            window.cell().unwrap().skip();
            let check = window
                .span(1, 3)
                .unwrap()
                .with_horz_align(CellAlign::Start)
                .wrap(CheckButton::default())
                .with_label("Save password");
            check.set_checked(!password.is_empty());
            check
        });

        window
            .row()
            .with_default_align(CellAlign::End)
            .with_stretch(1)
            .add();
        window.span(1, 2).unwrap().skip();
        let ok_button = window
            .cell()
            .unwrap()
            .wrap(ReturnButton::default())
            .with_label("OK");
        let mut cancel_button = window
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Cancel");

        let window_grid = window.end();
        let mut window = window_grid.group();
        let window_size = window_grid.min_size();
        window.set_size(
            std::cmp::max(window.w(), window_size.width),
            window_size.height,
        );
        window_grid.layout_children();

        window.set_pos(
            parent.x() + (parent.w() - window.w()) / 2,
            parent.y() + (parent.h() - window.h()) / 2,
        );

        cancel_button.set_callback({
            let mut window = window.clone();
            move |_| window.hide()
        });

        (
            window,
            server_text,
            password_text,
            save_password_check,
            ok_button,
        )
    }
}

const ERR_INVALID_ADDR: &str = "Invalid server address.";
