use std::borrow::Cow;
use std::rc::Rc;

use fltk::button::CheckButton;
use fltk::enums::{CallbackTrigger, Event};
use fltk::frame::Frame;
use fltk::input::Input;
use fltk::misc::InputChoice;
use fltk::prelude::*;
use fltk_float::grid::{CellAlign, Grid};
use fltk_float::LayoutElement;
use strum::IntoEnumIterator;

use crate::game::Maps;
use crate::gui::widgets::DropDownList;
use crate::gui::{glyph, wrapper_factory};
use crate::servers::{Mode, Region, TypeFilter};
use crate::util::weak_cb;

use super::state::Filter;
use super::{mode_name, region_name};

pub(super) trait FilterHolder {
    fn access_filter(&self, accessor: impl FnOnce(&Filter));
    fn mutate_filter(&self, mutator: impl FnMut(&mut Filter));
    fn persist_filter(&self);
}

pub(super) struct FilterPane {
    grid: Grid,
    name_input: Input,
    map_input: InputChoice,
    type_input: DropDownList,
    mode_input: DropDownList,
    region_input: DropDownList,
    battleye_input: DropDownList,
    invalid_check: CheckButton,
    pwd_prot_check: CheckButton,
    mods_input: DropDownList,
}

impl FilterPane {
    pub fn new(maps: &Maps) -> Rc<Self> {
        let mut grid = Grid::builder_with_factory(wrapper_factory())
            .with_col_spacing(10)
            .with_row_spacing(10);
        grid.col().with_default_align(CellAlign::End).add();
        grid.col().with_stretch(1).add();
        grid.col().with_default_align(CellAlign::End).add();
        grid.col().with_stretch(1).add();
        grid.col().with_default_align(CellAlign::End).add();
        grid.col().with_stretch(1).add();
        grid.col().add();

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Server Name:");
        let name_input = grid.span(1, 6).unwrap().wrap(Input::default());

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Map:");
        let mut map_input = grid.cell().unwrap().wrap(InputChoice::default());
        map_input.add("<clear map filter>");
        for map in maps.iter() {
            map_input.add(&map.display_name);
        }
        grid.cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Type:");
        let mut type_input = grid.cell().unwrap().wrap(DropDownList::default());
        for type_filter in TypeFilter::iter() {
            type_input.add(type_name(type_filter).as_ref());
        }
        type_input.set_value(0);
        grid.cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Mode:");
        let mut mode_input = grid.cell().unwrap().wrap(DropDownList::default());
        mode_input.add("All");
        for mode in Mode::iter() {
            mode_input.add(mode_name(mode));
        }
        mode_input.set_value(0);
        let invalid_check = grid
            .cell()
            .unwrap()
            .wrap(CheckButton::default())
            .with_label(&format!("{} Show invalid servers", glyph::ERROR));

        grid.row().add();
        grid.cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Region:");
        let mut region_input = grid.cell().unwrap().wrap(DropDownList::default());
        region_input.add("All");
        for region in Region::iter() {
            region_input.add(region_name(region));
        }
        region_input.set_value(0);
        grid.cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("Mods:");
        let mut mods_input = grid.cell().unwrap().wrap(DropDownList::default());
        mods_input.add("All");
        mods_input.add("Unmodded");
        mods_input.add(&format!("Modded {}", glyph::TOOLS));
        mods_input.set_value(0);
        grid.cell()
            .unwrap()
            .wrap(Frame::default())
            .with_label("BattlEye:");
        let mut battleye_input = grid.cell().unwrap().wrap(DropDownList::default());
        battleye_input.add("All");
        battleye_input.add(&format!("Required {}", glyph::BATTLEYE));
        battleye_input.add("Not Required");
        battleye_input.set_value(0);
        let pwd_prot_check = grid
            .cell()
            .unwrap()
            .wrap(CheckButton::default())
            .with_label(&format!("{} Show password protected servers", glyph::LOCK));

        let grid = grid.end();

        Rc::new(Self {
            grid,
            name_input,
            map_input,
            type_input,
            mode_input,
            region_input,
            battleye_input,
            invalid_check,
            pwd_prot_check,
            mods_input,
        })
    }

    pub fn set_filter_holder(&self, filter_holder: Rc<impl FilterHolder + 'static>) {
        filter_holder.access_filter(|filter| self.populate(filter));
        self.set_callbacks(filter_holder);
    }

    fn populate(&self, filter: &Filter) {
        self.name_input.clone().set_value(filter.name());
        self.map_input.clone().set_value(filter.map());
        self.type_input
            .clone()
            .set_value(filter.type_filter() as u8);
        self.mode_input.clone().set_value(match filter.mode() {
            Some(mode) => (mode as i32) + 1,
            None => 0,
        });
        self.region_input.clone().set_value(match filter.region() {
            Some(region) => (region as i32) + 1,
            None => 0,
        });
        self.battleye_input
            .clone()
            .set_value(match filter.battleye_required() {
                None => 0,
                Some(true) => 1,
                Some(false) => 2,
            });
        self.invalid_check
            .clone()
            .set_checked(filter.include_invalid());
        self.pwd_prot_check
            .clone()
            .set_checked(filter.include_password_protected());
        self.mods_input.clone().set_value(match filter.mods() {
            None => 0,
            Some(false) => 1,
            Some(true) => 2,
        });
    }

    fn set_callbacks(&self, filter_holder: Rc<impl FilterHolder + 'static>) {
        {
            let mut name_input = self.name_input.clone();
            name_input.set_trigger(CallbackTrigger::Changed);
            name_input.set_callback(weak_cb!(
                [filter_holder] => |input| {
                    filter_holder.mutate_filter(|filter| filter.set_name(input.value()));
                }
            ));
            set_unfocus_handler(&mut name_input, &filter_holder);
        }
        {
            let mut map_input = self.map_input.clone();
            map_input.set_trigger(CallbackTrigger::Changed);
            map_input.set_callback(weak_cb!(
                [filter_holder] => |input| {
                    if input.menu_button().value() == 0 {
                        input.set_value("");
                    }
                    input.menu_button().set_value(-1);
                    filter_holder
                        .mutate_filter(|filter| filter.set_map(input.value().unwrap_or_default()));
                }
            ));
            set_unfocus_handler(&mut map_input, &filter_holder);
        }

        let mut type_input = self.type_input.clone();
        type_input.set_callback(weak_cb!(
            [filter_holder] => |input| {
                let repr = input.value();
                let type_filter = TypeFilter::from_repr(repr as _).unwrap();
                filter_holder.mutate_filter(|filter| filter.set_type_filter(type_filter));
                filter_holder.persist_filter();
            }
        ));

        let mut mode_input = self.mode_input.clone();
        mode_input.set_callback(weak_cb!(
            [filter_holder] => |input| {
                let mode = {
                    let repr = input.value() - 1;
                    if repr < 0 {
                        None
                    } else {
                        Mode::from_repr(repr as _)
                    }
                };
                filter_holder.mutate_filter(|filter| filter.set_mode(mode));
                filter_holder.persist_filter();
            }
        ));

        let mut region_input = self.region_input.clone();
        region_input.set_callback(weak_cb!(
            [filter_holder] => |input| {
                let region = {
                    let repr = input.value() - 1;
                    if repr < 0 {
                        None
                    } else {
                        Region::from_repr(repr as _)
                    }
                };
                filter_holder.mutate_filter(|filter| filter.set_region(region));
                filter_holder.persist_filter();
            }
        ));

        let mut battleye_input = self.battleye_input.clone();
        battleye_input.set_callback(weak_cb!(
            [filter_holder] => |input| {
                let required = match input.value() {
                    1 => Some(true),
                    2 => Some(false),
                    _ => None,
                };
                filter_holder.mutate_filter(|filter| filter.set_battleye_required(required));
                filter_holder.persist_filter();
            }
        ));

        let mut invalid_check = self.invalid_check.clone();
        invalid_check.set_trigger(CallbackTrigger::Changed);
        invalid_check.set_callback(weak_cb!(
            [filter_holder] => |input| {
                filter_holder
                    .mutate_filter(|filter| filter.set_include_invalid(input.is_checked()));
                filter_holder.persist_filter();
            }
        ));

        let mut pwd_prot_check = self.pwd_prot_check.clone();
        pwd_prot_check.set_trigger(CallbackTrigger::Changed);
        pwd_prot_check.set_callback(weak_cb!(
            [filter_holder] => |input| {
                filter_holder.mutate_filter(|filter| {
                    filter.set_include_password_protected(input.is_checked())
                });
                filter_holder.persist_filter();
            }
        ));

        let mut mods_input = self.mods_input.clone();
        mods_input.set_callback(weak_cb!(
            [filter_holder] => |input| {
                let mods = match input.value() {
                    1 => Some(false),
                    2 => Some(true),
                    _ => None,
                };
                filter_holder.mutate_filter(|filter| filter.set_mods(mods));
                filter_holder.persist_filter();
            }
        ));
    }
}

impl LayoutElement for FilterPane {
    fn min_size(&self) -> fltk_float::Size {
        self.grid.min_size()
    }

    fn layout(&self, x: i32, y: i32, width: i32, height: i32) {
        self.grid.layout(x, y, width, height)
    }
}

fn type_name(type_filter: TypeFilter) -> Cow<'static, str> {
    match type_filter {
        TypeFilter::All => "All".into(),
        TypeFilter::Official => format!("Official {}", glyph::OFFICIAL).into(),
        TypeFilter::Private => "Private".into(),
        TypeFilter::Favorite => format!("Favorite {}", glyph::FAVORITE).into(),
    }
}

fn set_unfocus_handler<W: WidgetBase>(
    widget: &mut W,
    filter_holder: &Rc<impl FilterHolder + 'static>,
) {
    widget.handle(weak_cb!([filter_holder] => |_, event| {
        if let Event::Unfocus | Event::Hide = event {
            filter_holder.persist_filter();
        }
    }; false));
}
