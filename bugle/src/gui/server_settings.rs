pub mod dialog;
pub mod tabs;

use std::borrow::Borrow;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::Weekday;
use fltk::button::CheckButton;
use fltk::enums::{Align, CallbackTrigger, Event};
use fltk::frame::Frame;
use fltk::input::Input;
use fltk::prelude::*;
use fltk::valuator::HorNiceSlider;
use fltk_float::grid::{CellAlign, GridBuilder};
use fltk_float::{SimpleWrapper, WrapperFactory};

use crate::game::settings::{DailyHours, DailyHoursEntry, HourMinute, Hours, WeeklyHours};
use crate::util::weekday_iter;

use super::weekday_name;
use super::widgets::DropDownList;

#[derive(Clone)]
struct SliderInput {
    slider: HorNiceSlider,
    input: Input,
    precision: usize,
}

impl SliderInput {
    fn new(min: f64, max: f64, step: f64, step_div: i32) -> Self {
        let normalized_div = (step_div as f64) / step;
        let precision = normalized_div.log10().ceil() as usize;
        Self::with_precision(min, max, step, step_div, precision)
    }

    fn with_precision(min: f64, max: f64, step: f64, step_div: i32, precision: usize) -> Self {
        let mut slider = HorNiceSlider::default();
        slider.set_bounds(min, max);
        slider.set_step(step, step_div);

        let mut input = Input::default();

        slider.set_callback({
            let mut input = input.clone();
            move |slider| {
                input.set_value(&format!("{:.*}", precision, slider.value()));
            }
        });

        input.set_trigger(CallbackTrigger::Changed);
        input.set_callback({
            let mut slider = slider.clone();
            move |input| {
                if let Ok(value) = input.value().parse::<f64>() {
                    let value = value.clamp(slider.minimum(), slider.maximum());
                    slider.set_value(value);
                }
            }
        });
        input.handle({
            let slider = slider.clone();
            move |input, event| {
                if let Event::Unfocus | Event::Hide = event {
                    input.set_value(&format!("{:.*}", precision, slider.value()));
                }
                false
            }
        });

        Self {
            slider,
            input,
            precision,
        }
    }

    fn value(&self) -> f64 {
        self.slider.value()
    }

    fn set_value<V: Into<f64>>(&self, value: V) {
        let value = value
            .into()
            .clamp(self.slider.minimum(), self.slider.maximum());
        self.slider.clone().set_value(value);
        self.input
            .clone()
            .set_value(&format!("{:.*}", self.precision, value));
    }
}

#[derive(Clone)]
struct HoursInput {
    start_input: Input,
    end_input: Input,
    value: Rc<Cell<Hours>>,
}

impl HoursInput {
    fn new() -> Self {
        let init_value = Hours::default();
        let value = Rc::new(Cell::new(init_value));

        let mut start_input = Input::default();
        start_input.set_value(&init_value.start.to_string());
        start_input.set_trigger(CallbackTrigger::Changed);
        start_input.set_callback({
            let value = Rc::clone(&value);
            move |input| {
                if let Ok(parsed) = input.value().parse::<HourMinute>() {
                    let mut new_value = value.get();
                    new_value.start = parsed;
                    value.set(new_value);
                }
            }
        });
        start_input.handle({
            let value = Rc::clone(&value);
            move |input, event| {
                if let Event::Unfocus | Event::Hide = event {
                    input.set_value(&value.get().start.to_string());
                }
                false
            }
        });

        let mut end_input = Input::default();
        end_input.set_value(&init_value.end.to_string());
        end_input.set_trigger(CallbackTrigger::Changed);
        end_input.set_callback({
            let value = Rc::clone(&value);
            move |input| {
                if let Ok(parsed) = input.value().parse::<HourMinute>() {
                    let mut new_value = value.get();
                    new_value.end = parsed;
                    value.set(new_value);
                }
            }
        });
        end_input.handle({
            let value = Rc::clone(&value);
            move |input, event| {
                if let Event::Unfocus | Event::Hide = event {
                    input.set_value(&value.get().end.to_string());
                }
                false
            }
        });

        Self {
            start_input,
            end_input,
            value,
        }
    }

    fn value(&self) -> Hours {
        self.value.get()
    }

    fn set_value(&self, value: Hours) {
        self.value.set(value);
        self.start_input.clone().set_value(&value.start.to_string());
        self.end_input.clone().set_value(&value.end.to_string());
    }
}

struct DailyHoursInput(HashMap<Weekday, (CheckButton, HoursInput)>);

impl DailyHoursInput {
    fn value(&self) -> DailyHours {
        self.0
            .iter()
            .map(|(day, (enabled, hours))| {
                (
                    *day,
                    DailyHoursEntry {
                        enabled: enabled.is_checked(),
                        hours: hours.value(),
                    },
                )
            })
            .collect()
    }

    fn set_value(&self, value: &DailyHours) {
        for (day, entry) in value.iter() {
            let (enabled, hours) = &self.0[&day];
            enabled.set_checked(entry.enabled);
            hours.set_value(entry.hours);
        }
    }
}

struct WeeklyHoursInput {
    weekday: HoursInput,
    weekend: HoursInput,
}

impl WeeklyHoursInput {
    fn new() -> Self {
        Self {
            weekday: HoursInput::new(),
            weekend: HoursInput::new(),
        }
    }

    fn value(&self) -> WeeklyHours {
        WeeklyHours {
            weekday_hours: self.weekday.value(),
            weekend_hours: self.weekend.value(),
        }
    }

    fn set_value(&self, value: &WeeklyHours) {
        self.weekday.set_value(value.weekday_hours);
        self.weekend.set_value(value.weekend_hours);
    }
}

fn make_label(text: &str) -> Frame {
    Frame::default()
        .with_label(text)
        .with_align(Align::Left | Align::Inside)
}

trait EditorBuilder {
    type BoolProp;
    type RangeProp;
    type EnumProp;
    type DailyHoursProp;
    type WeeklyHoursProp;

    fn bool_prop(&mut self, label: &str) -> Self::BoolProp;
    fn range_prop(
        &mut self,
        label: &str,
        min: f64,
        max: f64,
        step: f64,
        step_div: i32,
    ) -> Self::RangeProp;
    fn enum_prop(&mut self, label: &str, values: &[&str]) -> Self::EnumProp;
    fn daily_hours_prop(&mut self, check_label: &str) -> Self::DailyHoursProp;
    fn weekly_hours_prop(&mut self) -> Self::WeeklyHoursProp;
}

impl<G: GroupExt + Clone, F: Borrow<WrapperFactory>> EditorBuilder for GridBuilder<G, F> {
    type BoolProp = CheckButton;
    type RangeProp = SliderInput;
    type EnumProp = DropDownList;
    type DailyHoursProp = DailyHoursInput;
    type WeeklyHoursProp = WeeklyHoursInput;

    fn bool_prop(&mut self, label: &str) -> Self::BoolProp {
        let span_cols = self.num_cols();
        self.row().add();
        let button = self
            .span(1, span_cols)
            .unwrap()
            .wrap(CheckButton::default().with_label(label));
        button
    }

    fn range_prop(
        &mut self,
        label: &str,
        min: f64,
        max: f64,
        step: f64,
        step_div: i32,
    ) -> Self::RangeProp {
        let span_cols = self.num_cols() - 2;
        self.row().add();
        let slider_input = SliderInput::new(min, max, step, step_div);
        self.cell().unwrap().wrap(make_label(label));
        self.span(1, span_cols)
            .unwrap()
            .with_vert_align(CellAlign::Stretch)
            .add(SimpleWrapper::new(
                slider_input.slider.clone(),
                Default::default(),
            ));
        self.cell().unwrap().wrap(slider_input.input.clone());
        slider_input
    }

    fn enum_prop(&mut self, label: &str, values: &[&str]) -> Self::EnumProp {
        let span_cols = self.num_cols() - 1;
        self.row().add();
        self.cell().unwrap().wrap(make_label(label));
        let mut input = self
            .span(1, span_cols)
            .unwrap()
            .wrap(DropDownList::default());
        for value in values {
            input.add(value);
        }
        input
    }

    fn daily_hours_prop(&mut self, check_label: &str) -> Self::DailyHoursProp {
        let mut result = HashMap::with_capacity(7);
        for day in weekday_iter() {
            self.row().add();
            self.cell().unwrap().wrap(
                Frame::default()
                    .with_label(weekday_name(day))
                    .with_align(Align::Right | Align::Inside),
            );
            let enabled_check = self
                .cell()
                .unwrap()
                .wrap(CheckButton::default().with_label(check_label));
            let input = HoursInput::new();
            self.cell()
                .unwrap()
                .wrap(Frame::default().with_label("Start:"));
            self.cell().unwrap().wrap(input.start_input.clone());
            self.cell()
                .unwrap()
                .wrap(Frame::default().with_label("End:"));
            self.cell().unwrap().wrap(input.end_input.clone());

            result.insert(day, (enabled_check, input));
        }
        DailyHoursInput(result)
    }

    fn weekly_hours_prop(&mut self) -> Self::WeeklyHoursProp {
        let input = WeeklyHoursInput::new();

        let span_cols = self.num_cols() - 4;
        self.row().add();
        self.span(1, span_cols)
            .unwrap()
            .wrap(make_label("    Weekdays (Mon-Fri)"));
        self.cell()
            .unwrap()
            .wrap(Frame::default().with_label("Start:"));
        self.cell().unwrap().wrap(input.weekday.start_input.clone());
        self.cell()
            .unwrap()
            .wrap(Frame::default().with_label("End:"));
        self.cell().unwrap().wrap(input.weekday.end_input.clone());

        self.row().add();
        self.span(1, span_cols)
            .unwrap()
            .wrap(make_label("    Weekends (Sat-Sun)"));
        self.cell()
            .unwrap()
            .wrap(Frame::default().with_label("Start:"));
        self.cell().unwrap().wrap(input.weekend.start_input.clone());
        self.cell()
            .unwrap()
            .wrap(Frame::default().with_label("End:"));
        self.cell().unwrap().wrap(input.weekend.end_input.clone());

        input
    }
}

struct PrivateBuilder<B: EditorBuilder> {
    public: B,
    build_private: bool,
}

impl<B: EditorBuilder> PrivateBuilder<B> {
    fn new(public: B, build_private: bool) -> Self {
        Self {
            public,
            build_private,
        }
    }

    fn into_inner(self) -> B {
        self.public
    }
}

impl<B: EditorBuilder> EditorBuilder for PrivateBuilder<B> {
    type BoolProp = Option<B::BoolProp>;
    type RangeProp = Option<B::RangeProp>;
    type EnumProp = Option<B::EnumProp>;
    type DailyHoursProp = Option<B::DailyHoursProp>;
    type WeeklyHoursProp = Option<B::WeeklyHoursProp>;

    fn bool_prop(&mut self, label: &str) -> Self::BoolProp {
        self.build_private.then(|| self.public.bool_prop(label))
    }

    fn range_prop(
        &mut self,
        label: &str,
        min: f64,
        max: f64,
        step: f64,
        step_div: i32,
    ) -> Self::RangeProp {
        self.build_private
            .then(|| self.public.range_prop(label, min, max, step, step_div))
    }

    fn enum_prop(&mut self, label: &str, values: &[&str]) -> Self::EnumProp {
        self.build_private
            .then(|| self.public.enum_prop(label, values))
    }

    fn daily_hours_prop(&mut self, check_label: &str) -> Self::DailyHoursProp {
        self.build_private
            .then(|| self.public.daily_hours_prop(check_label))
    }

    fn weekly_hours_prop(&mut self) -> Self::WeeklyHoursProp {
        self.build_private.then(|| self.public.weekly_hours_prop())
    }
}
