use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use fltk::button::{Button, CheckButton, ReturnButton, ToggleButton};
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::input::Input;
use fltk::misc::InputChoice;
use fltk::prelude::*;
use fltk::window::Window;
use fltk_float::grid::{CellAlign, Grid};
use fltk_float::LayoutElement;
use strum::IntoEnumIterator;

use crate::game::settings::server::CombatModeModifier;
use crate::game::Game;
use crate::gui::server_settings::tabs::SettingsTabs;
use crate::gui::widgets::DropDownList;
use crate::gui::{alert_error, wrapper_factory};
use crate::servers::{Mode, Ownership, Region, Server, ServerData};
use crate::util::weak_cb;

use super::{mode_name, region_name};

pub struct AddServerDialog {
    build_id: u32,
    window: Window,
    name_input: Input,
    host_input: Input,
    map_input: InputChoice,
    mode_input: DropDownList,
    region_input: DropDownList,
    pwd_prot_check: CheckButton,
    settings_tabs: SettingsTabs,
    result: RefCell<Option<Server>>,
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
        let mut battleye_check = root
            .cell()
            .unwrap()
            .wrap(CheckButton::default())
            .with_label("Requires BattlEye");

        root.row().with_stretch(1).add();
        let mut settings_grid = Grid::builder_with_factory(wrapper_factory());
        let settings_tabs = SettingsTabs::new(
            &mut settings_grid,
            false,
            &Default::default(),
            &Default::default(),
        );
        let settings_grid = settings_grid.end();
        let mut settings_group = settings_grid.group();
        settings_group.hide();
        let min_tabs_height = settings_grid.min_size().height;
        root.span(1, 5)
            .unwrap()
            .with_horz_align(CellAlign::Stretch)
            .with_vert_align(CellAlign::Stretch)
            .add(CollapsibleWrapper::new(settings_grid, Default::default()));

        root.row().add();
        let mut btn_grid = Grid::builder_with_factory(wrapper_factory()).with_col_spacing(10);
        btn_grid.row().add();
        btn_grid.col().add();
        btn_grid.col().with_stretch(1).add();
        let btn_group = btn_grid.col_group().add();
        btn_grid.extend_group(btn_group).batch(2);
        let mut settings_button = btn_grid
            .cell()
            .unwrap()
            .wrap(ToggleButton::default())
            .with_label(LABEL_EXPAND_SETTINGS);
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

        settings_button.set_callback({
            let width = window.w();
            let collapsed_height = min_size.height;
            let expanded_height = std::cmp::max(600, collapsed_height + min_tabs_height);
            let mut window = window.clone();
            move |settings_button| {
                if settings_group.visible() {
                    settings_button.set_label(LABEL_EXPAND_SETTINGS);
                    settings_group.hide();
                    window.set_size(width, collapsed_height);
                    root.layout(0, 0, window.w(), window.h());
                } else {
                    settings_button.set_label(LABEL_COLLAPSE_SETTINGS);
                    window.set_size(width, expanded_height);
                    root.layout(0, 0, window.w(), window.h());
                    settings_group.show();
                }
            }
        });

        let this = Rc::new(Self {
            build_id: game.build_id(),
            window,
            name_input,
            host_input,
            map_input,
            mode_input: mode_input.clone(),
            region_input,
            pwd_prot_check,
            settings_tabs,
            result: RefCell::new(None),
        });

        ok_button.set_callback(weak_cb!([this] => |_| this.ok_clicked()));
        cancel_button.set_callback(weak_cb!([this] => |_| this.cancel_clicked()));
        mode_input.set_callback(weak_cb!([this] => |input| {
            let Some(mode) = Mode::from_repr(input.value() as _) else {
                return;
            };
            this.settings_tabs.general_tab.set_mode(mode);
        }));
        battleye_check.set_callback(weak_cb!([this] => |input| {
            this.settings_tabs.general_tab.set_battleye_required(input.is_checked());
        }));
        this.settings_tabs
            .general_tab
            .set_pvp_enabled_callback(weak_cb!([this] => |enabled| {
                let mode = if enabled {
                    match this.settings_tabs.general_tab.mode_modifier() {
                        CombatModeModifier::Conflict => Mode::PVEC,
                        _ => Mode::PVP,
                    }
                } else {
                    Mode::PVE
                };
                this.mode_input.clone().set_value(mode as u8);
            }));

        this
    }

    pub fn run(&self) -> Option<Server> {
        let mut window = self.window.clone();
        window.make_modal(true);
        window.show();

        while window.shown() && !fltk::app::should_program_quit() {
            fltk::app::wait();
        }

        self.result.borrow_mut().take()
    }

    fn ok_clicked(&self) {
        let server = match self.make_server() {
            Ok(server) => server,
            Err(err) => {
                alert_error(ERR_INVALID_SERVER_DATA, &err);
                return;
            }
        };
        *self.result.borrow_mut() = Some(server);
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
            region,
            max_players: 0,
            reported_ip: host.ip(),
            observed_ip: None,
            port: host.port() as _,
            build_id: self.build_id,
            mods: None,
            general: self.settings_tabs.general_tab.public_values(),
            progression: self.settings_tabs.progression_tab.public_values(),
            daylight: self.settings_tabs.daylight_tab.public_values(),
            survival: self.settings_tabs.survival_tab.public_values(),
            combat: self.settings_tabs.combat_tab.public_values(),
            harvesting: self.settings_tabs.harvesting_tab.public_values(),
            crafting: self.settings_tabs.crafting_tab.public_values(),
        });

        Ok(server)
    }
}

const ERR_INVALID_SERVER_DATA: &str = "Invalid server data.";

const LABEL_EXPAND_SETTINGS: &str = "Settings @2>>";
const LABEL_COLLAPSE_SETTINGS: &str = "Settings @8>>";

struct CollapsibleWrapper<E: LayoutElement> {
    element: E,
    collapsed_size: fltk_float::Size,
}

impl<E: LayoutElement> CollapsibleWrapper<E> {
    pub fn new(element: E, collapsed_size: fltk_float::Size) -> Self {
        Self {
            element,
            collapsed_size,
        }
    }
}

impl<E: LayoutElement> LayoutElement for CollapsibleWrapper<E> {
    fn min_size(&self) -> fltk_float::Size {
        self.collapsed_size
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.element.layout(x, y, width, height)
    }
}
