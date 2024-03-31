use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use chrono::{TimeDelta, Weekday};
use ini::Properties;
use ini_persist::load::{LoadProperty, ParseProperty};
use ini_persist::save::{DisplayProperty, SaveProperty};
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

use crate::util::weekday_iter;

pub mod server;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Multiplier(f64);

impl Multiplier {
    pub fn to_string(&self) -> String {
        format!("{:.2}", self.0)
    }
}

impl Default for Multiplier {
    fn default() -> Self {
        Self(1.0)
    }
}

impl ParseProperty for Multiplier {
    fn parse(text: &str) -> ini_persist::Result<Self> {
        Ok(Self(f64::parse(text)?))
    }
}

impl DisplayProperty for Multiplier {
    fn display(&self) -> String {
        format!("{}", self.0)
    }
}

impl FromStr for Multiplier {
    type Err = <f64 as FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Multiplier(s.parse()?))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HourMinute(pub u16);

impl HourMinute {
    pub fn hours(self) -> u8 {
        (self.0 / 100) as _
    }

    pub fn minutes(self) -> u8 {
        (self.0 % 100) as _
    }

    pub fn to_string(self) -> String {
        format!("{:02}:{:02}", self.hours(), self.minutes())
    }
}

impl From<u16> for HourMinute {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl ParseProperty for HourMinute {
    fn parse(text: &str) -> ini_persist::Result<Self> {
        Ok(Self(u16::parse(text)?))
    }
}

impl DisplayProperty for HourMinute {
    fn display(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Debug, Clone, Copy, Default, LoadProperty, SaveProperty)]
pub struct Hours {
    #[ini(rename = "Start")]
    pub start: HourMinute,

    #[ini(rename = "End")]
    pub end: HourMinute,
}

#[derive(Debug, Clone, Default)]
pub struct DailyHours(HashMap<Weekday, Hours>);

impl DailyHours {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl Deref for DailyHours {
    type Target = HashMap<Weekday, Hours>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DailyHours {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<(Weekday, Hours)> for DailyHours {
    fn from_iter<T: IntoIterator<Item = (Weekday, Hours)>>(iter: T) -> Self {
        Self(HashMap::from_iter(iter))
    }
}

impl LoadProperty for DailyHours {
    fn load_in(&mut self, section: &Properties, key: &str) -> ini_persist::Result<()> {
        use ini_persist::load::ConstructProperty;
        self.clear();
        for day in weekday_iter() {
            let day_name = DAY_NAMES[day.num_days_from_monday() as usize];
            if bool::load(section, &format!("{}Enabled{}", key, day_name))?.unwrap_or_default() {
                let start = u16::load(section, &format!("{}Time{}Start", key, day_name))?;
                let end = u16::load(section, &format!("{}Time{}End", key, day_name))?;
                self.insert(
                    day,
                    Hours {
                        start: start.unwrap_or_default().into(),
                        end: end.unwrap_or_default().into(),
                    },
                );
            }
        }
        Ok(())
    }
}

impl SaveProperty for DailyHours {
    fn remove(section: &mut Properties, key: &str) {
        for day in weekday_iter() {
            let day_name = DAY_NAMES[day.num_days_from_monday() as usize];
            let _ = section.remove_all(format!("{}Enabled{}", key, day_name));
            let _ = section.remove_all(format!("{}Time{}Start", key, day_name));
            let _ = section.remove_all(format!("{}Time{}End", key, day_name));
        }
    }

    fn append(&self, section: &mut Properties, key: &str) {
        for day in weekday_iter() {
            let day_name = DAY_NAMES[day.num_days_from_monday() as usize];
            match self.0.get(&day) {
                Some(Hours { start, end }) => {
                    section.append(format!("{}Enabled{}", key, day_name), "True");
                    section.append(format!("{}Time{}Start", key, day_name), start.0.to_string());
                    section.append(format!("{}Time{}End", key, day_name), end.0.to_string());
                }
                None => {
                    section.append(format!("{}Enabled{}", key, day_name), "False");
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default, LoadProperty, SaveProperty)]
pub struct WeeklyHours {
    #[ini(rename = "Weekday")]
    pub weekday_hours: Hours,

    #[ini(rename = "Weekend")]
    pub weekend_hours: Hours,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, EnumIter, LoadProperty, SaveProperty,
)]
#[repr(u8)]
#[ini(repr)]
pub enum Nudity {
    None,
    Partial,
    Full,
}

fn parse_seconds(value: &str) -> ini_persist::Result<TimeDelta> {
    parse_delta(value, "seconds", 1.0)
}

fn parse_minutes(value: &str) -> ini_persist::Result<TimeDelta> {
    parse_delta(value, "minutes", 60.0)
}

fn parse_delta(value: &str, unit: &str, seconds_per_unit: f64) -> ini_persist::Result<TimeDelta> {
    let count = f64::parse(value)? * seconds_per_unit;
    let secs = count as i64;
    let nanos = (count.fract().abs() * NANOS_PER_SEC) as u32;
    TimeDelta::new(secs, nanos).ok_or_else(|| {
        ini_persist::Error::invalid_value(format!("interval out of range: {} {}", count, unit))
    })
}

fn display_seconds(value: &TimeDelta) -> String {
    display_delta(value, 1.0)
}

fn display_minutes(value: &TimeDelta) -> String {
    display_delta(value, 60.0)
}

fn display_delta(value: &TimeDelta, seconds_per_unit: f64) -> String {
    let seconds = (value.num_seconds() as f64) + (value.subsec_nanos() as f64) / NANOS_PER_SEC;
    format!("{}", seconds / seconds_per_unit)
}

const DAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const NANOS_PER_SEC: f64 = 1_000_000_000.0;
