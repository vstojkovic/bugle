use std::path::PathBuf;
use std::process::{Child, Command};

use fltk::app::{self, App};
use fltk::button::Button;
use fltk::dialog;
use fltk::enums::FrameType;
use fltk::group::Column;
use fltk::prelude::*;
use fltk::window::Window;
use steamlocate::SteamDir;

struct Game {
    root: PathBuf,
}

impl Game {
    fn locate() -> Option<Self> {
        let mut steam = SteamDir::locate()?;
        let app = steam.app(&440900)?;
        Some(Self {
            root: app.path.clone(),
        })
    }

    fn launch(&self, enable_battleye: bool, args: &[&str]) -> std::io::Result<Child> {
        let mut exe_path = self.root.clone();
        exe_path.extend(["ConanSandbox", "Binaries", "Win64"]);
        exe_path.push(if enable_battleye {
            "ConanSandbox_BE.exe"
        } else {
            "ConanSandbox.exe"
        });

        let mut cmd = Command::new(exe_path);
        cmd.args(args);
        if enable_battleye {
            cmd.arg("-BattlEye");
        }

        cmd.spawn()
    }
}

fn make_button<F: FnMut(&mut Button) + 'static>(text: &str, callback: F) -> Button {
    let mut button = Button::default().with_label(text);
    button.set_frame(FrameType::RoundUpBox);
    button.set_down_frame(FrameType::RoundDownBox);
    button.clear_visible_focus();
    button.set_callback(callback);
    button
}

fn not_implemented(_: &mut Button) {
    dialog::alert_default("This feature is not yet implemented in the current release.");
}

fn main() {
    let launcher = App::default();

    let game = std::rc::Rc::new({
        match Game::locate() {
            Some(game) => game,
            None => {
                dialog::alert_default(
                    "Cannot locate Conan Exiles installation. Please verify that you have Conan \
                    Exiles installed in a Steam library and try again.",
                );
                return;
            }
        }
    });

    let mut main_win = Window::default().with_size(400, 300);
    main_win.set_label("BUGLE");

    let mut vpack = Column::default_fill();
    vpack.set_margin(10);
    vpack.set_pad(10);

    let _continue_btn = {
        let game = game.clone();
        make_button("Continue", move |_| {
            match game.launch(true, &["-continuesession"]) {
                Ok(_) => app::quit(),
                Err(err) => {
                    dialog::alert_default(&format!("Failed to launch Conan Exiles:\n{}", err))
                }
            }
        })
    };
    let _online_btn = make_button("Online", not_implemented);
    let _sp_btn = make_button("Singleplayer", not_implemented);
    let _coop_btn = make_button("Co-op", not_implemented);
    let _mods_btn = make_button("Mods", not_implemented);
    let _settings_btn = make_button("Settings", not_implemented);
    let _exit_btn = make_button("Exit", |_| {
        app::quit();
    });

    vpack.end();

    main_win.end();
    main_win.show();

    launcher.run().unwrap();
}
