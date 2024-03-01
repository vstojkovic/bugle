use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use fltk::button::{Button, CheckButton, ReturnButton};
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::input::Input;
use fltk::misc::InputChoice;
use fltk::prelude::*;
use fltk::window::Window;
use fltk_float::grid::{CellAlign, Grid};
use fltk_float::LayoutElement;
use strum::IntoEnumIterator;

use crate::game::Game;
use crate::gui::prelude::declare_weak_cb;
use crate::gui::widgets::DropDownList;
use crate::gui::{alert_error, wrapper_factory};
use crate::servers::{Community, Kind, Mode, Ownership, Region, Server, ServerData};

use super::{mode_name, region_name, ServerBrowserAction};

pub struct AddServerDialog {
    build_id: u32,
    window: Window,
    name_input: Input,
    host_input: Input,
    map_input: InputChoice,
    mode_input: DropDownList,
    region_input: DropDownList,
    pwd_prot_check: CheckButton,
    battleye_check: CheckButton,
    result: RefCell<Option<ServerBrowserAction>>,
}

impl AddServerDialog {
    pub fn new(parent: &Group, game: Arc<Game>) -> Rc<Self> {
        let mut window = Window::default().with_size(0, 0).with_label("Add Server");

        let mut root = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10)
            .with_padding(10, 10, 10, 10);

        root.col().with_default_align(CellAlign::End).add();
        root.col().with_stretch(1).add();
        root.col().with_default_align(CellAlign::End).add();
        root.col().with_stretch(1).add();
        root.col().with_stretch(1).add();

        root.row().add();
        root.cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Name:");
        let name_input = root.span(1, 4).unwrap().wrap(Input::default());

        root.row().add();
        root.cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Host:");
        let host_input = root.cell().unwrap().wrap(Input::default());
        root.cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Map:");
        let mut map_input = root.cell().unwrap().wrap(InputChoice::default());
        for map in game.maps().iter() {
            map_input.add(&map.display_name);
        }
        let pwd_prot_check = root
            .cell()
            .unwrap()
            .wrap(CheckButton::default())
            .with_label("Password protected");

        root.row().add();
        root.cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Mode:");
        let mut mode_input = root.cell().unwrap().wrap(DropDownList::default());
        for mode in Mode::iter() {
            mode_input.add(mode_name(mode));
        }
        root.cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Region:");
        let mut region_input = root.cell().unwrap().wrap(DropDownList::default());
        for region in Region::iter() {
            region_input.add(region_name(region));
        }
        let battleye_check = root
            .cell()
            .unwrap()
            .wrap(CheckButton::default())
            .with_label("Requires BattlEye");

        root.row()
            .with_default_align(CellAlign::End)
            .with_stretch(1)
            .add();
        let mut btn_grid = Grid::builder_with_factory(wrapper_factory()).with_col_spacing(10);
        btn_grid.row().add();
        btn_grid.col().with_stretch(1).add();
        let btn_group = btn_grid.col_group().add();
        btn_grid.extend_group(btn_group).batch(2);
        btn_grid.cell().unwrap().skip();
        let mut ok_button = btn_grid
            .cell()
            .unwrap()
            .wrap(ReturnButton::default())
            .with_label("OK");
        let mut cancel_button = btn_grid
            .cell()
            .unwrap()
            .wrap(Button::default())
            .with_label("Cancel");
        root.span(1, 5)
            .unwrap()
            .with_horz_align(CellAlign::Stretch)
            .add(btn_grid.end());

        let root = root.end();
        let min_size = root.min_size();
        window.set_size(800, min_size.height);
        root.group().resize(0, 0, 800, min_size.height);
        root.layout_children();
        window.set_pos(
            parent.x() + (parent.w() - window.w()) / 2,
            parent.y() + (parent.h() - window.h()) / 2,
        );

        let this = Rc::new(Self {
            build_id: game.build_id(),
            window,
            name_input,
            host_input,
            map_input,
            mode_input,
            region_input,
            pwd_prot_check,
            battleye_check,
            result: RefCell::new(None),
        });

        ok_button.set_callback(this.weak_cb(Self::ok_clicked));
        cancel_button.set_callback(this.weak_cb(Self::cancel_clicked));

        this
    }

    pub fn run(&self) -> Option<ServerBrowserAction> {
        let mut window = self.window.clone();
        window.make_modal(true);
        window.show();

        while window.shown() {
            fltk::app::wait();
        }

        self.result.borrow_mut().take()
    }

    declare_weak_cb!();

    fn ok_clicked(&self) {
        let server = match self.make_server() {
            Ok(server) => server,
            Err(err) => {
                alert_error(ERR_INVALID_SERVER_DATA, &err);
                return;
            }
        };
        *self.result.borrow_mut() =
            Some(ServerBrowserAction::ToggleSavedServer { server, idx: None });
        self.window.clone().hide();
    }

    fn cancel_clicked(&self) {
        self.window.clone().hide();
    }

    fn make_server(&self) -> Result<Server> {
        let name = self.name_input.value();
        if name.is_empty() {
            bail!("Name cannot be empty.");
        }

        let host = SocketAddr::from_str(&self.host_input.value())
            .map_err(|err| anyhow!("Invalid host ({}).", err))?;

        let map = self
            .map_input
            .value()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("Map cannot be empty."))?;

        if self.mode_input.value() < 0 {
            bail!("Please select a mode.");
        }
        let mode = Mode::from_repr(self.mode_input.value() as _).unwrap();
        let kind = match mode {
            Mode::PVEC => Kind::Conflict,
            _ => Kind::Other,
        };

        if self.region_input.value() < 0 {
            bail!("Please select a region.");
        }
        let region = Region::from_repr(self.region_input.value() as _).unwrap();

        let server = Server::new(ServerData {
            id: "".to_string(),
            name,
            map,
            password_protected: self.pwd_prot_check.is_checked(),
            ownership: Ownership::Private,
            battleye_required: self.battleye_check.is_checked(),
            region,
            max_players: 0,
            pvp_enabled: mode != Mode::PVE,
            kind,
            reported_ip: host.ip(),
            observed_ip: None,
            port: host.port() as _,
            build_id: self.build_id,
            community: Community::Unspecified,
            mods: None,
            max_ping: None,
            max_clan_size: None,
            xp_rate_mult: Default::default(),
            daylight: Default::default(),
            survival: Default::default(),
            combat: Default::default(),
            harvesting: Default::default(),
            crafting: Default::default(),
            raid_hours: Default::default(),
        });

        Ok(server)
    }
}

const ERR_INVALID_SERVER_DATA: &str = "Invalid server data.";
