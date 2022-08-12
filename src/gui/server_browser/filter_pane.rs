use std::rc::Rc;

use fltk::button::CheckButton;
use fltk::enums::{Align, CallbackTrigger};
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::input::Input;
use fltk::misc::InputChoice;
use fltk::prelude::*;
use strum::IntoEnumIterator;

use crate::gui::prelude::*;
use crate::gui::{widget_auto_height, widget_col_width};
use crate::servers::{Filter, Mode, Region};

use super::{mode_name, region_name};

pub(super) trait FilterHolder {
    fn access_filter(&self, accessor: impl FnOnce(&Filter));
    fn mutate_filter(&self, mutator: impl FnMut(&mut Filter));
}

pub(super) struct FilterPane {
    root: Group,
    name_input: Input,
    map_input: Input,
    mode_input: InputChoice,
    region_input: InputChoice,
    invalid_check: CheckButton,
    pwd_prot_check: CheckButton,
    build_id: u32,
}

impl FilterPane {
    pub fn new(build_id: u32) -> Self {
        let mut root = Group::default_fill();
        let label_align = Align::Right | Align::Inside;
        let name_label = Frame::default()
            .with_label("Server Name:")
            .with_align(label_align);
        let map_label = Frame::default().with_label("Map:").with_align(label_align);
        let invalid_check = CheckButton::default().with_label("Show invalid servers");
        let mode_label = Frame::default().with_label("Mode:").with_align(label_align);
        let region_label = Frame::default()
            .with_label("Region:")
            .with_align(label_align);
        let pwd_prot_check = CheckButton::default().with_label("Show password protected servers");
        let left_width = widget_col_width(&[&name_label, &mode_label]);
        let mid_width = widget_col_width(&[&mode_label, &region_label]);
        let right_width = widget_col_width(&[&invalid_check, &pwd_prot_check]);
        let height = widget_auto_height(&name_label);
        let input_width = (root.w() - left_width - mid_width - right_width - 40) / 2;

        root.set_size(root.w(), height * 2 + 10);

        let name_label = name_label.with_size(left_width, height).inside_parent(0, 0);
        let name_input = Input::default()
            .with_size(input_width, height)
            .right_of(&name_label, 10);
        let map_label = map_label
            .with_size(mid_width, height)
            .right_of(&name_input, 10);
        let map_input = Input::default()
            .with_size(input_width, height)
            .right_of(&map_label, 10);
        let invalid_check = invalid_check
            .with_size(right_width, height)
            .right_of(&map_input, 10);
        let mode_label = mode_label
            .with_size(left_width, height)
            .below_of(&name_label, 10);
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
        let region_label = region_label
            .with_size(mid_width, height)
            .right_of(&mode_input, 10);
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
        let pwd_prot_check = pwd_prot_check
            .with_size(right_width, height)
            .right_of(&region_input, 10);

        root.end();

        Self {
            root,
            name_input,
            map_input,
            mode_input,
            region_input,
            invalid_check,
            pwd_prot_check,
            build_id,
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
        self.invalid_check
            .clone()
            .set_checked(filter.build_id().is_none());
        self.pwd_prot_check
            .clone()
            .set_checked(filter.password_protected());
    }

    fn set_callbacks(&self, filter_holder: Rc<impl FilterHolder + 'static>) {
        {
            let filter_holder = Rc::downgrade(&filter_holder);
            let mut name_input = self.name_input.clone();
            name_input.set_trigger(CallbackTrigger::Changed);
            name_input.set_callback(move |input| {
                if let Some(filter_holder) = filter_holder.upgrade() {
                    filter_holder.mutate_filter(|filter| filter.set_name(input.value()));
                }
            });
        }
        {
            let filter_holder = Rc::downgrade(&filter_holder);
            let mut map_input = self.map_input.clone();
            map_input.set_trigger(CallbackTrigger::Changed);
            map_input.set_callback(move |input| {
                if let Some(filter_holder) = filter_holder.upgrade() {
                    filter_holder.mutate_filter(|filter| filter.set_map(input.value()));
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
                    filter_holder.mutate_filter(|filter| filter.set_mode(mode));
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
                    filter_holder.mutate_filter(|filter| filter.set_region(region));
                }
            });
        }
        {
            let filter_holder = Rc::downgrade(&filter_holder);
            let build_id = self.build_id;
            let mut invalid_check = self.invalid_check.clone();
            invalid_check.set_trigger(CallbackTrigger::Changed);
            invalid_check.set_callback(move |input| {
                if let Some(filter_holder) = filter_holder.upgrade() {
                    let build_id = if input.is_checked() { None } else { Some(build_id) };
                    filter_holder.mutate_filter(|filter| filter.set_build_id(build_id));
                }
            })
        }
        {
            let filter_holder = Rc::downgrade(&filter_holder);
            let mut pwd_prot_check = self.pwd_prot_check.clone();
            pwd_prot_check.set_trigger(CallbackTrigger::Changed);
            pwd_prot_check.set_callback(move |input| {
                if let Some(filter_holder) = filter_holder.upgrade() {
                    filter_holder
                        .mutate_filter(|filter| filter.set_password_protected(input.is_checked()));
                }
            })
        }
    }
}
