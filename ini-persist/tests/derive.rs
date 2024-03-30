use ini::Ini;
use ini_persist::load::{IniLoad, LoadProperty};

#[derive(Debug, Default, PartialEq, IniLoad)]
struct Root {
    #[ini(general)]
    general: General,

    section: Section,

    #[ini(section = "SomethingElse")]
    renamed: RenamedSection,
}

#[derive(Debug, Default, PartialEq, LoadProperty)]
struct General {
    argle: String,

    #[ini(rename = "Bargle")]
    bargle: u8,

    #[ini(flatten)]
    inner: Inner,

    prefixed: Inner,
}

#[derive(Debug, Default, PartialEq, LoadProperty)]
struct Inner {
    #[ini(load_in_with = helpers::my_load_in)]
    glop: i32,

    glyf: Option<EnumByName>,
}

#[derive(Debug, Default, PartialEq, LoadProperty)]
struct Section {
    #[ini(load_with = helpers::my_load)]
    olle_bolle: f64,

    snop: Option<EnumByRepr>,
}

#[derive(Debug, Default, PartialEq, LoadProperty)]
struct RenamedSection {
    #[ini(parse_with = helpers::my_parse)]
    snyf: i16,
}

#[derive(Debug, LoadProperty, PartialEq, Eq)]
enum EnumByName {
    Argle,
    Bargle,
}

#[derive(Debug, LoadProperty, PartialEq, Eq)]
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
}

#[test]
fn compilation_errors() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/compile-fail/*.rs");
}

#[test]
fn comprehensive_test() {
    let ini = Ini::load_from_str(TEST_INI).unwrap();
    let mut loaded = Root::default();
    loaded.load_from_ini(&ini).unwrap();

    let expected = Root {
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
    };

    assert_eq!(loaded, expected);
}

const TEST_INI: &str = r#"
argle=Hello, world!
Bargle=17
glop=123
glyf=Bargle
prefixedglop=456,
prefixedglyf=Argle

[section]
olle_bolle=84.0
snop=17

[SomethingElse]
snyf=-42
"#;
