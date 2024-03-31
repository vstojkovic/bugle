use ini::{Ini, WriteOption};
use ini_persist::load::{IniLoad, LoadProperty};
use ini_persist::save::{IniSave, SaveProperty};

#[derive(Debug, Default, PartialEq, IniLoad, IniSave)]
struct Root {
    #[ini(general)]
    general: General,

    section: Section,

    #[ini(section = "SomethingElse")]
    renamed: RenamedSection,
}

#[derive(Debug, Default, PartialEq, LoadProperty, SaveProperty)]
struct General {
    argle: String,

    #[ini(rename = "Bargle", remove_with = helpers::my_remove)]
    bargle: u8,

    #[ini(flatten)]
    inner: Inner,

    prefixed: Inner,
}

#[derive(Debug, Default, PartialEq, LoadProperty, SaveProperty)]
struct Inner {
    #[ini(load_in_with = helpers::my_load_in, append_with = helpers::my_append)]
    glop: i32,

    glyf: Option<EnumByName>,
}

#[derive(Debug, Default, PartialEq, LoadProperty, SaveProperty)]
struct Section {
    #[ini(load_with = helpers::my_load, display_with = helpers::my_display_f64)]
    olle_bolle: f64,

    snop: Option<EnumByRepr>,
}

#[derive(Debug, Default, PartialEq, LoadProperty, SaveProperty)]
struct RenamedSection {
    #[ini(parse_with = helpers::my_parse, display_with = helpers::my_display_i16)]
    snyf: i16,
}

#[derive(Debug, LoadProperty, SaveProperty, PartialEq, Eq)]
enum EnumByName {
    Argle,
    Bargle,
}

#[derive(Debug, LoadProperty, SaveProperty, PartialEq, Eq)]
#[ini(repr)]
#[repr(u8)]
enum EnumByRepr {
    Glop = 42,
    Glyf = 17,
}

mod helpers {
    use ini::Properties;
    use ini_persist::load::ParseProperty;
    use ini_persist::Result;

    pub fn my_load_in(field: &mut i32, _section: &Properties, _key: &str) -> Result<()> {
        *field = 386;
        Ok(())
    }

    pub fn my_load(section: &Properties, key: &str) -> Result<Option<f64>> {
        match section.get(key) {
            Some(value) => Ok(Some(0.5 * f64::parse(value)?)),
            None => Ok(None),
        }
    }

    pub fn my_parse(value: &str) -> Result<i16> {
        Ok(-i16::parse(value)?)
    }

    pub fn my_append(_field: &i32, section: &mut Properties, key: &str) {
        let value = if key == "glop" { 123 } else { 456 };
        section.append(key, value.to_string());
    }

    pub fn my_display_f64(field: &f64) -> String {
        format!("{:.2}", *field * 2.0)
    }

    pub fn my_display_i16(field: &i16) -> String {
        format!("{}", -*field)
    }

    pub fn my_remove(section: &mut Properties, key: &str) {
        let _ = section.remove_all(key);
        let _ = section.remove_all("quux");
    }
}

#[test]
fn compilation_errors() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/compile-fail/*.rs");
}

#[test]
fn comprehensive_loading_test() {
    let ini = Ini::load_from_str(TEST_INI).unwrap();
    let mut loaded = Root::default();
    loaded.load_from_ini(&ini).unwrap();

    let expected = make_test_data();
    assert_eq!(loaded, expected);
}

#[test]
fn comprehensive_saving_test() {
    use std::io::Write;

    let mut ini = Ini::new();
    ini.with_section(None::<String>)
        .set("quux", format!("{}", 420.17));

    let to_save = make_test_data();
    to_save.save_to_ini(&mut ini);

    let mut saved = vec![];
    write!(&mut saved, "\n").unwrap();
    ini.write_to_opt(
        &mut saved,
        WriteOption {
            escape_policy: ini::EscapePolicy::Nothing,
            line_separator: ini::LineSeparator::CR,
        },
    )
    .unwrap();
    let saved = String::from_utf8(saved).unwrap();

    assert_eq!(saved, TEST_INI);
}

fn make_test_data() -> Root {
    Root {
        general: General {
            argle: "Hello, world!".to_string(),
            bargle: 17,
            inner: Inner {
                glop: 386,
                glyf: Some(EnumByName::Bargle),
            },
            prefixed: Inner {
                glop: 386,
                glyf: Some(EnumByName::Argle),
            },
        },
        section: Section {
            olle_bolle: 42.0,
            snop: Some(EnumByRepr::Glyf),
        },
        renamed: RenamedSection { snyf: 42 },
    }
}

const TEST_INI: &str = r#"
argle=Hello, world!
Bargle=17
glop=123
glyf=Bargle
prefixedglop=456
prefixedglyf=Argle

[section]
olle_bolle=84.00
snop=17

[SomethingElse]
snyf=-42
"#;
