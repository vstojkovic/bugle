use std::collections::HashMap;
use std::error::Error;
use std::fmt::Display;

use nom::branch::alt;
use nom::bytes::complete::{escaped, escaped_transform, is_not, take_while_m_n};
use nom::character::complete::{anychar, char, multispace0};
use nom::combinator::{all_consuming, map, recognize};
use nom::error::ErrorKind;
use nom::multi::{fold_many0, many1_count};
use nom::sequence::{delimited, preceded};
use nom::{AsChar, IResult};

#[derive(Debug)]
pub struct ParserError {
    kind: ErrorKind,
    offset: isize,
}

impl ParserError {
    pub fn from_error(input: &str, err: nom::error::Error<&str>) -> Self {
        let offset = unsafe { err.input.as_ptr().offset_from(input.as_ptr()) };
        Self {
            kind: err.code,
            offset,
        }
    }

    pub fn from_err(input: &str, err: nom::Err<nom::error::Error<&str>>) -> Self {
        let err = match err {
            nom::Err::Error(err) => err,
            nom::Err::Failure(err) => err,
            _ => unreachable!(),
        };
        Self::from_error(input, err)
    }
}

impl Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error {:?} at: {}", self.kind, self.offset)
    }
}

impl Error for ParserError {}

pub fn parse_map(input: &str) -> IResult<&str, HashMap<&str, &str>> {
    all_consuming(delimited(char('('), parse_map_entries, char(')')))(input)
}

fn parse_map_entries(input: &str) -> IResult<&str, HashMap<&str, &str>> {
    let (input, (key, value)) = parse_entry(input)?;
    fold_many0(
        preceded(char(','), parse_entry),
        move || {
            let mut map = HashMap::new();
            map.insert(key, value);
            map
        },
        |mut map, (key, value)| {
            map.entry(key).or_insert(value);
            map
        },
    )(input)
}

pub fn parse_entry(input: &str) -> IResult<&str, (&str, &str)> {
    let (input, key) = parse_key(input)?;
    let (input, _) = char('=')(input)?;
    let (input, value) = parse_value(input)?;
    Ok((input, (key, value)))
}

pub fn parse_key(input: &str) -> IResult<&str, &str> {
    map(preceded(multispace0, is_not("=")), str::trim_end)(input)
}

pub fn parse_value(input: &str) -> IResult<&str, &str> {
    map(
        preceded(
            multispace0,
            recognize(many1_count(alt((
                is_not("\",)"),
                delimited(char('"'), escaped(is_not("\\\""), '\\', anychar), char('"')),
            )))),
        ),
        str::trim_end,
    )(input)
}

pub fn parse_quoted(input: &str) -> IResult<&str, String> {
    delimited(
        char('"'),
        escaped_transform(is_not("\\\""), '\\', anychar),
        char('"'),
    )(input)
}

pub fn parse_hex(input: &str, len: usize) -> IResult<&str, &str> {
    all_consuming(take_while_m_n(len, len, AsChar::is_hex_digit))(input)
}

pub fn extract_value<V>(result: IResult<&str, V>) -> Result<V, nom::Err<nom::error::Error<&str>>> {
    result.map(|(_, value)| value)
}
