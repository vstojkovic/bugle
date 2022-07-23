use std::path::PathBuf;
use std::process::{Child, Command};

use fltk::app::{self, App};
use fltk::dialog;
use steamlocate::SteamDir;

mod gui;
mod servers;

use gui::LauncherWindow;

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

#[tokio::main]
async fn main() {
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

    let on_continue = {
        let game = game.clone();
        move || {
            let _ = game.launch(true, &["-continuesession"])?;
            app::quit();
            Ok(())
        }
    };

    let mut main_win = LauncherWindow::new(on_continue);
    main_win.show();

    launcher.run().unwrap();
}
