use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

use fltk::button::CheckButton;
use fltk::enums::{Align, CallbackTrigger};
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::input::Input;
use fltk::misc::InputChoice;
use fltk::prelude::*;
use strum::IntoEnumIterator;

use crate::game::Maps;
use crate::gui::{glyph, prelude::*};
use crate::gui::{widget_auto_height, widget_col_width};
use crate::servers::{Mode, Region, TypeFilter};

use super::state::Filter;
use super::{mode_name, region_name};

pub(super) trait FilterHolder {
    fn access_filter(&self, accessor: impl FnOnce(&Filter));
    fn mutate_filter(&self, change: FilterChange, mutator: impl FnMut(&mut Filter));
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum FilterChange {
    Name,
    Map,
    Type,
    Mode,
    Region,
    BattlEyeRequired,
    IncludeInvalid,
    IncludePasswordProtected,
    Mods,
}

pub(super) struct FilterPane {
    root: Group,
    name_input: Input,
    map_input: InputChoice,
    type_input: InputChoice,
    mode_input: InputChoice,
    region_input: InputChoice,
    battleye_input: InputChoice,
    invalid_check: CheckButton,
    pwd_prot_check: CheckButton,
    mods_input: InputChoice,
}

impl FilterPane {
    pub fn new(maps: Arc<Maps>) -> Self {
        let mut root = Group::default_fill();
        let label_align = Align::Right | Align::Inside;
        let name_label = Frame::default()
            .with_label("Server Name:")
            .with_align(label_align);
        let map_label = Frame::default().with_label("Map:").with_align(label_align);
        let invalid_check =
            CheckButton::default().with_label(&format!("{} Show invalid servers", glyph::WARNING));
        let type_label = Frame::default().with_label("Type:").with_align(label_align);
        let mode_label = Frame::default().with_label("Mode:").with_align(label_align);
        let pwd_prot_check = CheckButton::default()
            .with_label(&format!("{} Show password protected servers", glyph::LOCK));
        let region_label = Frame::default()
            .with_label("Region:")
            .with_align(label_align);
        let mods_label = Frame::default().with_label("Mods:").with_align(label_align);
        let battleye_label = Frame::default()
            .with_label("BattlEye:")
            .with_align(label_align);
        let left_width = widget_col_width(&[&name_label, &map_label, &region_label]);
        let mid_width = widget_col_width(&[&type_label, &mods_label]);
        let right_width = widget_col_width(&[&mode_label, &battleye_label]);
        let check_width = widget_col_width(&[&invalid_check, &pwd_prot_check]);
        let height = widget_auto_height(&name_label);
        let input_width = (root.w() - left_width - mid_width - right_width - check_width - 60) / 3;

        root.set_size(root.w(), height * 3 + 20);

        let name_label = name_label.with_size(left_width, height).inside_parent(0, 0);
        let name_input = Input::default()
            .with_size(root.w() - left_width - 10, height)
            .right_of(&name_label, 10);
        let map_label = map_label
            .with_size(left_width, height)
            .below_of(&name_label, 10);
        let mut map_input = InputChoice::default()
            .with_size(input_width, height)
            .right_of(&map_label, 10);
        for map in maps.iter() {
            map_input.add(&map.display_name);
        }
        let type_label = type_label
            .with_size(mid_width, height)
            .right_of(&map_input, 10);
        let mut type_input = InputChoice::default()
            .with_size(input_width, height)
            .right_of(&type_label, 10);
        type_input.input().set_readonly(true);
        type_input.input().clear_visible_focus();
        for type_filter in TypeFilter::iter() {
            type_input.add(type_name(type_filter).as_ref());
        }
        type_input.set_value_index(0);
        let mode_label = mode_label
            .with_size(right_width, height)
            .right_of(&type_input, 10);
        let mut mode_input = InputChoice::default()
            .with_size(input_width, height)
            .right_of(&mode_label, 10);
        mode_input.input().set_readonly(true);
        mode_input.input().clear_visible_focus();
        mode_input.add("All");
        for mode in Mode::iter() {
            mode_input.add(mode_name(mode));
        }
        mode_input.set_value_index(0);
        let invalid_check = invalid_check
            .with_size(check_width, height)
            .right_of(&mode_input, 10);
        let region_label = region_label
            .with_size(left_width, height)
            .below_of(&map_label, 10);
        let mut region_input = InputChoice::default()
            .with_size(input_width, height)
            .right_of(&region_label, 10);
        region_input.input().set_readonly(true);
        region_input.input().clear_visible_focus();
        region_input.add("All");
        for region in Region::iter() {
            region_input.add(region_name(region));
        }
        region_input.set_value_index(0);
        let mods_label = mods_label
            .with_size(mid_width, height)
            .right_of(&region_input, 10);
        let mut mods_input = InputChoice::default()
            .with_size(input_width, height)
            .right_of(&mods_label, 10);
        mods_input.input().set_readonly(true);
        mods_input.input().clear_visible_focus();
        mods_input.add("All");
        mods_input.add("Unmodded");
        mods_input.add(&format!("Modded {}", glyph::TOOLS));
        let battleye_label = battleye_label
            .with_size(right_width, height)
            .right_of(&mods_input, 10);
        let mut battleye_input = InputChoice::default()
            .with_size(input_width, height)
            .right_of(&battleye_label, 10);
        battleye_input.input().set_readonly(true);
        battleye_input.input().clear_visible_focus();
        battleye_input.add("All");
        battleye_input.add(&format!("Required {}", glyph::EYE));
        battleye_input.add("Not Required");
        battleye_input.set_value_index(0);
        let pwd_prot_check = pwd_prot_check
            .with_size(check_width, height)
            .right_of(&battleye_input, 10);

        root.end();

        Self {
            root,
            name_input,
            map_input,
            type_input,
            mode_input,
            region_input,
            battleye_input,
            invalid_check,
            pwd_prot_check,
            mods_input,
        }
    }

    pub fn root(&self) -> &Group {
        &self.root
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
            .set_value_index(filter.type_filter() as _);
        self.mode_input
            .clone()
            .set_value_index(match filter.mode() {
                Some(mode) => (mode as i32) + 1,
                None => 0,
            });
        self.region_input
            .clone()
            .set_value_index(match filter.region() {
                Some(region) => (region as i32) + 1,
                None => 0,
            });
        self.battleye_input
            .clone()
            .set_value_index(match filter.battleye_required() {
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
        self.mods_input
            .clone()
            .set_value_index(match filter.mods() {
                None => 0,
                Some(false) => 1,
                Some(true) => 2,
            });
    }

    fn set_callbacks(&self, filter_holder: Rc<impl FilterHolder + 'static>) {
        {
            let filter_holder = Rc::downgrade(&filter_holder);
            let mut name_input = self.name_input.clone();
            name_input.set_trigger(CallbackTrigger::Changed);
            name_input.set_callback(move |input| {
                if let Some(filter_holder) = filter_holder.upgrade() {
                    filter_holder
                        .mutate_filter(FilterChange::Name, |filter| filter.set_name(input.value()));
                }
            });
        }
        {
            let filter_holder = Rc::downgrade(&filter_holder);
            let mut map_input = self.map_input.clone();
            map_input.set_trigger(CallbackTrigger::Changed);
            map_input.set_callback(move |input| {
                if let Some(filter_holder) = filter_holder.upgrade() {
                    filter_holder.mutate_filter(FilterChange::Map, |filter| {
                        filter.set_map(input.value().unwrap_or_default())
                    });
                }
            });
        }
        {
            let filter_holder = Rc::downgrade(&filter_holder);
            let mut type_input = self.type_input.clone();
            type_input.set_trigger(CallbackTrigger::Changed);
            type_input.set_callback(move |input| {
                if let Some(filter_holder) = filter_holder.upgrade() {
                    let repr = input.menu_button().value();
                    let type_filter = TypeFilter::from_repr(repr as _).unwrap();
                    filter_holder.mutate_filter(FilterChange::Type, |filter| {
                        filter.set_type_filter(type_filter)
                    });
                }
            });
        }
        {
            let filter_holder = Rc::downgrade(&filter_holder);
            let mut mode_input = self.mode_input.clone();
            mode_input.set_trigger(CallbackTrigger::Changed);
            mode_input.set_callback(move |input| {
                if let Some(filter_holder) = filter_holder.upgrade() {
                    let mode = {
                        let repr = input.menu_button().value() - 1;
                        if repr < 0 {
                            None
                        } else {
                            Mode::from_repr(repr as _)
                        }
                    };
                    filter_holder.mutate_filter(FilterChange::Mode, |filter| filter.set_mode(mode));
                }
            });
        }
        {
            let filter_holder = Rc::downgrade(&filter_holder);
            let mut region_input = self.region_input.clone();
            region_input.set_trigger(CallbackTrigger::Changed);
            region_input.set_callback(move |input| {
                if let Some(filter_holder) = filter_holder.upgrade() {
                    let region = {
                        let repr = input.menu_button().value() - 1;
                        if repr < 0 {
                            None
                        } else {
                            Region::from_repr(repr as _)
                        }
                    };
                    filter_holder
                        .mutate_filter(FilterChange::Region, |filter| filter.set_region(region));
                }
            });
        }
        {
            let filter_holder = Rc::downgrade(&filter_holder);
            let mut battleye_input = self.battleye_input.clone();
            battleye_input.set_trigger(CallbackTrigger::Changed);
            battleye_input.set_callback(move |input| {
                if let Some(filter_holder) = filter_holder.upgrade() {
                    let required = match input.menu_button().value() {
                        1 => Some(true),
                        2 => Some(false),
                        _ => None,
                    };
                    filter_holder.mutate_filter(FilterChange::BattlEyeRequired, |filter| {
                        filter.set_battleye_required(required)
                    });
                }
            });
        }
        {
            let filter_holder = Rc::downgrade(&filter_holder);
            let mut invalid_check = self.invalid_check.clone();
            invalid_check.set_trigger(CallbackTrigger::Changed);
            invalid_check.set_callback(move |input| {
                if let Some(filter_holder) = filter_holder.upgrade() {
                    filter_holder.mutate_filter(FilterChange::IncludeInvalid, |filter| {
                        filter.set_include_invalid(input.is_checked())
                    });
                }
            })
        }
        {
            let filter_holder = Rc::downgrade(&filter_holder);
            let mut pwd_prot_check = self.pwd_prot_check.clone();
            pwd_prot_check.set_trigger(CallbackTrigger::Changed);
            pwd_prot_check.set_callback(move |input| {
                if let Some(filter_holder) = filter_holder.upgrade() {
                    filter_holder.mutate_filter(FilterChange::IncludePasswordProtected, |filter| {
                        filter.set_include_password_protected(input.is_checked())
                    });
                }
            })
        }
        {
            let filter_holder = Rc::downgrade(&filter_holder);
            let mut mods_input = self.mods_input.clone();
            mods_input.set_trigger(CallbackTrigger::Changed);
            mods_input.set_callback(move |input| {
                if let Some(filter_holder) = filter_holder.upgrade() {
                    let mods = match input.menu_button().value() {
                        1 => Some(false),
                        2 => Some(true),
                        _ => None,
                    };
                    filter_holder.mutate_filter(FilterChange::Mods, |filter| filter.set_mods(mods));
                }
            })
        }
    }
}

fn type_name(type_filter: TypeFilter) -> Cow<'static, str> {
    match type_filter {
        TypeFilter::All => "All".into(),
        TypeFilter::Official => format!("Official {}", glyph::FLAG).into(),
        TypeFilter::Private => "Private".into(),
        TypeFilter::Favorite => format!("Favorite {}", glyph::HEART).into(),
    }
}
