use std::fmt::Display;
use std::ops::{Index, IndexMut};
use std::str::FromStr;

use chrono::{TimeDelta, Weekday};
use ini::Properties;
use ini_persist::load::{LoadProperty, ParseProperty};
use ini_persist::save::{DisplayProperty, SaveProperty};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, FromRepr};

use crate::util::weekday_iter;

pub mod server;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Multiplier(pub f64);

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

impl From<Multiplier> for f64 {
    fn from(value: Multiplier) -> Self {
        value.0
    }
}

impl From<f64> for Multiplier {
    fn from(value: f64) -> Self {
        Self(value)
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
}

impl Display for HourMinute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.hours(), self.minutes())
    }
}

impl FromStr for HourMinute {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let [h, m] = HOUR_MINUTE_REGEX
            .captures(s)
            .map(|c| c.extract().1)
            .unwrap_or_default();
        let h: u16 = h.parse()?;
        let m: u16 = m.parse()?;
        Ok(HourMinute(h * 100 + m))
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
pub struct DailyHours([DailyHoursEntry; 7]);

#[derive(Debug, Clone, Default)]
pub struct DailyHoursEntry {
    pub enabled: bool,
    pub hours: Hours,
}

impl DailyHours {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&self, day: Weekday) -> Option<&Hours> {
        let entry = &self[day];
        if entry.enabled {
            Some(&entry.hours)
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Weekday, &DailyHoursEntry)> {
        weekday_iter().map(|day| (day, &self[day]))
    }
}

impl Index<Weekday> for DailyHours {
    type Output = DailyHoursEntry;
    fn index(&self, index: Weekday) -> &Self::Output {
        &self.0[index.num_days_from_monday() as usize]
    }
}

impl IndexMut<Weekday> for DailyHours {
    fn index_mut(&mut self, index: Weekday) -> &mut Self::Output {
        &mut self.0[index.num_days_from_monday() as usize]
    }
}

impl FromIterator<(Weekday, DailyHoursEntry)> for DailyHours {
    fn from_iter<T: IntoIterator<Item = (Weekday, DailyHoursEntry)>>(iter: T) -> Self {
        let mut result = DailyHours::new();
        for (day, entry) in iter {
            result[day] = entry;
        }
        result
    }
}

impl LoadProperty for DailyHours {
    fn load_in(&mut self, section: &Properties, key: &str) -> ini_persist::Result<()> {
        use ini_persist::load::ConstructProperty;
        for day in weekday_iter() {
            let day_name = weekday_name(day);
            let enabled =
                bool::load(section, &format!("{}Enabled{}", key, day_name))?.unwrap_or_default();
            let start = u16::load(section, &format!("{}Time{}Start", key, day_name))?;
            let end = u16::load(section, &format!("{}Time{}End", key, day_name))?;
            self[day] = DailyHoursEntry {
                enabled,
                hours: Hours {
                    start: start.unwrap_or_default().into(),
                    end: end.unwrap_or_default().into(),
                },
            };
        }
        Ok(())
    }
}

impl SaveProperty for DailyHours {
    fn remove(section: &mut Properties, key: &str) {
        for day in weekday_iter() {
            let day_name = weekday_name(day);
            let _ = section.remove_all(format!("{}Enabled{}", key, day_name));
            let _ = section.remove_all(format!("{}Time{}Start", key, day_name));
            let _ = section.remove_all(format!("{}Time{}End", key, day_name));
        }
    }

    fn append(&self, section: &mut Properties, key: &str) {
        for (day, entry) in self.iter() {
            let day_name = weekday_name(day);
            entry
                .enabled
                .append(section, &format!("{}Enabled{}", key, day_name));
            entry
                .hours
                .start
                .append(section, &format!("{}Time{}Start", key, day_name));
            entry
                .hours
                .end
                .append(section, &format!("{}Time{}End", key, day_name));
        }
    }
}

fn weekday_name(day: Weekday) -> &'static str {
    DAY_NAMES[day.num_days_from_monday() as usize]
}

#[derive(Debug, Clone, Default, LoadProperty, SaveProperty)]
pub struct WeeklyHours {
    #[ini(rename = "Weekday")]
    pub weekday_hours: Hours,

    #[ini(rename = "Weekend")]
    pub weekend_hours: Hours,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumIter,
    FromRepr,
    LoadProperty,
    SaveProperty,
)]
#[repr(u8)]
#[ini(repr)]
pub enum Nudity {
    None,
    Partial,
    Full,
}

impl Default for Nudity {
    fn default() -> Self {
        Self::None
    }
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

lazy_static! {
    static ref HOUR_MINUTE_REGEX: Regex = Regex::new(r"^(\d\d?):(\d\d)$").unwrap();
}
