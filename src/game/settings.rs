use std::collections::HashMap;

use chrono::Weekday;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Copy)]
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

pub type WeeklyHours = HashMap<Weekday, (HourMinute, HourMinute)>;
