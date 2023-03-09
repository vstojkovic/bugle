use std::fs::File;
use std::io::Write;
use std::rc::Rc;

use anyhow::Ok;
use fltk::browser::Browser;
use fltk::button::Button;
use fltk::enums::Font;
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::prelude::*;
use tempfile::tempdir;

use crate::game::Game;

use super::prelude::*;
use super::{button_row_height, widget_auto_height, widget_col_width};
use super::{Action, CleanupFn, Handler};

pub struct Home {
    root: Group,
}

impl Home {
    pub fn new(game: &Game, on_action: impl Handler<Action> + 'static) -> Rc<Self> {
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

        let info_pane = Browser::default_fill();

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

        let info_pane = info_pane.below_of(&btm_welcome_line, 10);
        let info_height = launch_button.y() - info_pane.y() - 10;
        let mut info_pane = info_pane.with_size_flex(0, info_height);

        info_pane.set_column_widths(&[info_pane.w() / 4, 3 * info_pane.w() / 4]);
        info_pane.set_column_char('\t');
        info_pane.add(&format!("BUGLE Version:\t{}", env!("CARGO_PKG_VERSION")));
        info_pane.add(&format!("Conan Exiles Build ID:\t{}", game.build_id()));
        info_pane.add({
            let (maj, min) = game.revision();
            &format!("Conan Exiles Revision:\t#{}/{}", maj, min)
        });
        info_pane.add(&format!(
            "Conan Exiles Installation Path:\t{}",
            game.installation_path().display()
        ));

        let on_action = Rc::new(on_action);
        launch_button.set_callback({
            let on_action = Rc::clone(&on_action);
            move |_| on_action(Action::Launch).unwrap()
        });
        continue_button.set_callback({
            let on_action = Rc::clone(&on_action);
            move |_| on_action(Action::Continue).unwrap()
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
