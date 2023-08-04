use std::borrow::Borrow;
use std::rc::Rc;

use fltk::app;
use fltk::button::{Button, RadioButton};
use fltk::dialog;
use fltk::enums::FrameType;
use fltk::prelude::*;
use fltk_float::grid::{CellAlign, Grid, GridBuilder};
use fltk_float::WrapperFactory;

use crate::l10n::{localization, use_l10n, Localizer};

use super::wrapper_factory;

pub(super) struct MainMenu {
    home_btn: RadioButton,
    online_btn: RadioButton,
    single_player_btn: RadioButton,
    mods_btn: RadioButton,
}

impl MainMenu {
    pub fn new() -> (Self, Grid) {
        let localizer = localization().localizer("main_menu");
        use_l10n!(localizer);

        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10);
        grid.col().with_stretch(1).add();

        let mut home_btn = make_button(&mut grid, RadioButton::default, l10n!(&home));
        let online_btn = make_button(&mut grid, RadioButton::default, l10n!(&online));
        let single_player_btn = make_button(&mut grid, RadioButton::default, l10n!(&sp));
        let mut coop_btn = make_button(&mut grid, Button::default, l10n!(&coop));
        let mods_btn = make_button(&mut grid, RadioButton::default, l10n!(&mods));
        let mut exit_btn = make_button(&mut grid, Button::default, l10n!(&exit));

        home_btn.toggle(true);

        let grid = grid.end();

        coop_btn.set_callback(not_implemented_callback(&localizer));
        exit_btn.set_callback(|_| app::quit());

        let menu = Self {
            home_btn,
            online_btn,
            single_player_btn,
            mods_btn,
        };

        (menu, grid)
    }

    pub fn set_on_home(&mut self, mut on_home: impl FnMut() + 'static) {
        self.home_btn.set_callback(move |_| on_home());
    }

    pub fn set_on_online(&mut self, mut on_online: impl FnMut() + 'static) {
        self.online_btn.set_callback(move |_| on_online());
    }

    pub fn set_on_single_player(&mut self, mut on_single_player: impl FnMut() + 'static) {
        self.single_player_btn
            .set_callback(move |_| on_single_player());
    }

    pub fn set_on_mods(&mut self, mut on_mods: impl FnMut() + 'static) {
        self.mods_btn.set_callback(move |_| on_mods());
    }
}

fn make_button<G, F, B, C>(grid: &mut GridBuilder<G, F>, ctor: C, text: &str) -> B
where
    G: GroupExt + Clone,
    F: Borrow<WrapperFactory>,
    B: ButtonExt + Clone + 'static,
    C: FnOnce() -> B,
{
    grid.row().with_stretch(1).add();

    let mut button = grid
        .cell()
        .unwrap()
        .with_vert_align(CellAlign::Stretch)
        .wrap(ctor().with_label(text));
    button.set_frame(FrameType::PlasticThinUpBox);
    button.set_down_frame(FrameType::PlasticThinDownBox);
    button.clear_visible_focus();
    button
}

fn not_implemented_callback<W: WidgetExt>(localizer: &Rc<Localizer>) -> impl Fn(&mut W) {
    let localizer = Rc::clone(localizer);
    move |_| {
        use_l10n!(localizer);
        dialog::alert_default(l10n!(&not_implemented));
    }
}
