use std::fs::File;
use std::io::Write;
use std::rc::Rc;

use anyhow::Ok;
use fltk::button::Button;
use fltk::enums::{CallbackTrigger, Font};
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::misc::InputChoice;
use fltk::prelude::*;
use fltk::table::TableRow;
use fltk_table::{SmartTable, TableOpts};
use tempfile::tempdir;

use crate::config::{BattlEyeUsage, Config};
use crate::game::Game;

use super::prelude::*;
use super::{
    alert_error, button_row_height, make_readonly_cell_widget, widget_auto_height,
    widget_auto_width, widget_col_width, Action, CleanupFn, Handler,
};

pub struct Home {
    root: Group,
}

impl Home {
    pub fn new(
        game: &Game,
        config: &Config,
        on_action: impl Handler<Action> + 'static,
    ) -> Rc<Self> {
        let on_action = Rc::new(on_action);

        let mut root = Group::default_fill();

        let top_welcome_line = Frame::default_fill().with_label("Welcome to");
        let top_welcome_height = widget_auto_height(&top_welcome_line);
        let top_welcome_line = top_welcome_line
            .with_size_flex(0, top_welcome_height)
            .inside_parent(0, 0);

        let mut mid_welcome_line = Frame::default_fill().with_label("BUGLE");
        mid_welcome_line.set_label_font(install_crom_font());
        mid_welcome_line.set_label_size(mid_welcome_line.label_size() * 3);
        let mid_welcome_height = widget_auto_height(&mid_welcome_line);
        let mid_welcome_line = mid_welcome_line
            .with_size_flex(0, mid_welcome_height)
            .below_of(&top_welcome_line, 0);

        let btm_welcome_line =
            Frame::default_fill().with_label("Butt-Ugly Game Launcher for Exiles");
        let btm_welcome_height = widget_auto_height(&btm_welcome_line);
        let btm_welcome_line = btm_welcome_line
            .with_size_flex(0, btm_welcome_height)
            .below_of(&mid_welcome_line, 0);

        let info_pane = SmartTable::default_fill().with_opts(TableOpts {
            rows: 4,
            cols: 1,
            editable: false,
            ..Default::default()
        });

        let battleye_label = Frame::default().with_label("BattlEye:");
        let battleye_height = widget_auto_height(&battleye_label);
        let battleye_group = Group::default_fill();
        let battleye_input = InputChoice::default_fill();
        battleye_group.end();

        let launch_button = Button::default().with_label("Launch");
        let continue_button = Button::default().with_label("Continue");
        let button_width = 2 * widget_col_width(&[&launch_button, &continue_button]);
        let button_height = 2 * button_row_height(&[&launch_button, &continue_button]);

        let mut continue_button = continue_button
            .with_size(button_width, button_height)
            .inside_parent(-button_width, -button_height);
        let mut launch_button = launch_button
            .with_size(button_width, button_height)
            .left_of(&continue_button, 10);

        let battleye_label_width = widget_auto_width(&battleye_label);
        let battleye_label = battleye_label
            .with_size(battleye_label_width, button_height)
            .inside_parent(0, -button_height);

        let battleye_group = battleye_group.right_of(&battleye_label, 10);
        let battleye_group_width = launch_button.x() - battleye_group.x() - button_width;
        let _battleye_group = battleye_group.with_size(battleye_group_width, button_height);

        let mut battleye_input = battleye_input
            .with_size_flex(0, battleye_height)
            .center_of_parent();
        battleye_input.input().set_readonly(true);
        battleye_input.input().clear_visible_focus();
        battleye_input.add("Enabled");
        battleye_input.add("Disabled");
        battleye_input.add("Only when required");
        battleye_input.set_value_index(match config.use_battleye {
            BattlEyeUsage::Always(true) => 0,
            BattlEyeUsage::Always(false) => 1,
            BattlEyeUsage::Auto => 2,
        });
        battleye_input.set_trigger(CallbackTrigger::Changed);
        battleye_input.set_callback({
            let on_action = Rc::clone(&on_action);
            move |input| {
                let use_battleye = match input.menu_button().value() {
                    0 => BattlEyeUsage::Always(true),
                    1 => BattlEyeUsage::Always(false),
                    2 => BattlEyeUsage::Auto,
                    _ => unreachable!(),
                };
                on_action(Action::ConfigureBattlEye(use_battleye)).unwrap();
            }
        });

        let mut info_pane = info_pane.below_of(&btm_welcome_line, 10);
        let info_height = launch_button.y() - info_pane.y() - 10;
        {
            let tbl: &TableRow = &info_pane;
            let _ = tbl.clone().with_size_flex(0, info_height);
        }

        let info_header_width = info_pane.w() / 4;
        let info_value_width = info_pane.w() - info_header_width - 10;
        info_pane.set_col_header(false);
        info_pane.set_row_header_width(info_header_width);
        info_pane.set_col_width(0, info_value_width);

        info_pane.set_row_header_value(0, "BUGLE Version");
        info_pane.set_cell_value(0, 0, env!("CARGO_PKG_VERSION"));
        info_pane.set_row_header_value(1, "Conan Exiles Build ID");
        info_pane.set_cell_value(1, 0, &format!("{}", game.build_id()));
        info_pane.set_row_header_value(2, "Conan Exiles Revision");
        info_pane.set_cell_value(2, 0, {
            let (maj, min) = game.revision();
            &format!("{}/{}", maj, min)
        });
        info_pane.set_row_header_value(3, "Conan Exiles Installation Path");
        info_pane.set_cell_value(3, 0, &game.installation_path().to_string_lossy());

        let _info_cell = make_readonly_cell_widget(&info_pane);

        launch_button.set_callback({
            let on_action = Rc::clone(&on_action);
            move |_| {
                if let Err(err) = on_action(Action::Launch) {
                    alert_error(ERR_LAUNCHING_GAME, &err);
                }
            }
        });
        continue_button.set_callback({
            let on_action = Rc::clone(&on_action);
            move |_| {
                if let Err(err) = on_action(Action::Continue) {
                    alert_error(ERR_LAUNCHING_GAME, &err);
                }
            }
        });

        root.end();
        root.hide();

        Rc::new(Self { root })
    }

    pub fn show(&self) -> CleanupFn {
        let mut root = self.root.clone();
        root.show();

        Box::new(move || {
            root.hide();
        })
    }
}

const ERR_LAUNCHING_GAME: &str = "Error while trying to launch the game.";

fn install_crom_font() -> Font {
    try_install_crom_font().unwrap_or(Font::TimesBold)
}

fn try_install_crom_font() -> anyhow::Result<Font> {
    let dir = tempdir()?;
    let path = dir.path().join("Crom_v1.ttf");

    let mut file = File::create(&path)?;
    file.write_all(include_bytes!("Crom_v1.ttf"))?;
    drop(file);

    let font = Font::load_font(path)?;
    Font::set_font(Font::Zapfdingbats, &font);
    Ok(Font::Zapfdingbats)
}
