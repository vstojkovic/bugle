mod building;
mod chat;
mod combat;
mod crafting;
mod daylight;
mod dialog;
mod followers;
mod general;
mod harvesting;
mod maelstrom;
mod progression;
mod survival;

use std::borrow::Borrow;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::Weekday;
pub use dialog::ServerSettingsDialog;
use fltk::button::CheckButton;
use fltk::enums::{Align, CallbackTrigger, Event};
use fltk::frame::Frame;
use fltk::input::Input;
use fltk::prelude::*;
use fltk::valuator::HorNiceSlider;
use fltk_float::grid::{CellAlign, GridBuilder};
use fltk_float::{SimpleWrapper, WrapperFactory};

use crate::game::settings::{DailyHours, HourMinute, Hours, WeeklyHours};
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

    fn set_value(&self, value: f64) {
        let value = value.clamp(self.slider.minimum(), self.slider.maximum());
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
    fn new(init_value: &Hours) -> Self {
        let value = Rc::new(Cell::new(*init_value));

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

    fn set_enabled(&self, enabled: bool) {
        let mut start_input = self.start_input.clone();
        let mut end_input = self.end_input.clone();
        if enabled {
            let value = self.value.get();
            start_input.activate();
            start_input.set_value(&value.start.to_string());
            end_input.activate();
            end_input.set_value(&value.end.to_string());
        } else {
            start_input.deactivate();
            start_input.set_value("");
            end_input.deactivate();
            end_input.set_value("");
        }
    }

    fn value(&self) -> Hours {
        self.value.get()
    }
}

struct DailyHoursInput(HashMap<Weekday, (CheckButton, HoursInput)>);

impl DailyHoursInput {
    fn value(&self) -> DailyHours {
        self.0
            .iter()
            .filter(|(_, (enabled, _))| enabled.is_checked())
            .map(|(day, (_, hours))| (*day, hours.value()))
            .collect()
    }
}

struct WeeklyHoursInput {
    weekday: HoursInput,
    weekend: HoursInput,
}

impl WeeklyHoursInput {
    fn new(init_value: &WeeklyHours) -> Self {
        Self {
            weekday: HoursInput::new(&init_value.weekday_hours),
            weekend: HoursInput::new(&init_value.weekend_hours),
        }
    }

    fn value(&self) -> WeeklyHours {
        WeeklyHours {
            weekday_hours: self.weekday.value(),
            weekend_hours: self.weekend.value(),
        }
    }
}

fn min_input_width(samples: &[&str]) -> i32 {
    fltk::draw::set_font(fltk::enums::Font::Helvetica, fltk::app::font_size());
    samples
        .into_iter()
        .map(|text| fltk::draw::measure(&format!("#{}#", text), false).0)
        .max()
        .unwrap_or_default()
}

fn make_label(text: &str) -> Frame {
    Frame::default()
        .with_label(text)
        .with_align(Align::Left | Align::Inside)
}

trait EditorBuilder {
    fn bool_prop<I: Into<bool>>(&mut self, label: &str, init_value: I) -> CheckButton;
    fn range_prop<I: Into<f64>>(
        &mut self,
        label: &str,
        min: f64,
        max: f64,
        step: f64,
        step_div: i32,
        init_value: I,
    ) -> SliderInput;
    fn enum_prop<I: Into<i32>>(
        &mut self,
        label: &str,
        values: &[&str],
        init_value: I,
    ) -> DropDownList;
    fn daily_hours_prop(&mut self, check_label: &str, init_value: &DailyHours) -> DailyHoursInput;
    fn weekly_hours_prop(&mut self, init_value: &WeeklyHours) -> WeeklyHoursInput;
}

impl<G: GroupExt + Clone, F: Borrow<WrapperFactory>> EditorBuilder for GridBuilder<G, F> {
    fn bool_prop<I: Into<bool>>(&mut self, label: &str, init_value: I) -> CheckButton {
        let span_cols = self.num_cols();
        self.row().add();
        let button = self
            .span(1, span_cols)
            .unwrap()
            .wrap(CheckButton::default().with_label(label));
        button.set_checked(init_value.into());
        button
    }

    fn range_prop<I: Into<f64>>(
        &mut self,
        label: &str,
        min: f64,
        max: f64,
        step: f64,
        step_div: i32,
        init_value: I,
    ) -> SliderInput {
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
        slider_input.set_value(init_value.into());
        slider_input
    }

    fn enum_prop<I: Into<i32>>(
        &mut self,
        label: &str,
        values: &[&str],
        init_value: I,
    ) -> DropDownList {
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
        input.set_value(init_value.into());
        input
    }

    fn daily_hours_prop(&mut self, check_label: &str, init_value: &DailyHours) -> DailyHoursInput {
        let mut result = HashMap::with_capacity(7);
        for day in weekday_iter() {
            let (enabled, value) = match init_value.get(&day) {
                Some(value) => (true, *value),
                None => (false, Hours::default()),
            };

            self.row().add();
            self.cell().unwrap().wrap(
                Frame::default()
                    .with_label(weekday_name(day))
                    .with_align(Align::Right | Align::Inside),
            );
            let mut enabled_check = self
                .cell()
                .unwrap()
                .wrap(CheckButton::default().with_label(check_label));
            let input = HoursInput::new(&value);
            self.cell()
                .unwrap()
                .wrap(Frame::default().with_label("Start:"));
            self.cell().unwrap().wrap(input.start_input.clone());
            self.cell()
                .unwrap()
                .wrap(Frame::default().with_label("End:"));
            self.cell().unwrap().wrap(input.end_input.clone());

            enabled_check.set_checked(enabled);
            input.set_enabled(enabled);

            enabled_check.set_callback({
                let input = input.clone();
                move |check| input.set_enabled(check.is_checked())
            });

            result.insert(day, (enabled_check, input));
        }
        DailyHoursInput(result)
    }

    fn weekly_hours_prop(&mut self, init_value: &WeeklyHours) -> WeeklyHoursInput {
        let input = WeeklyHoursInput::new(init_value);

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
