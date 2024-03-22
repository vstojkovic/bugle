use ini::Ini;
use ini_persist::load::IniLoad;
use ini_persist::IniLoad;

#[derive(Debug, Default, PartialEq, IniLoad)]
#[ini(general)]
struct Root {
    argle: String,

    #[ini(key = "Bargle")]
    bargle: u8,

    #[ini(flatten)]
    section: Section,
}

#[derive(Debug, Default, PartialEq, IniLoad)]
struct Section {
    #[ini(load_in_with = helpers::my_load_in)]
    glop_glyf: i32,

    #[ini(flatten)]
    general: FlattenedGeneral,

    #[ini(flatten)]
    renamed: RenamedSection,
}

#[derive(Debug, Default, PartialEq, IniLoad)]
#[ini(general)]
struct FlattenedGeneral {
    #[ini(load_with = helpers::my_load)]
    olle_bolle: f64,
}

#[derive(Debug, Default, PartialEq, IniLoad)]
#[ini(section = "SomethingElse")]
struct RenamedSection {
    #[ini(parse_with = helpers::my_parse)]
    snop_snyf: i16,
}

mod helpers {
    use ini::Properties;
    use ini_persist::load::ParsedProperty;
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
        argle: "Hello, world!".to_string(),
        bargle: 17,
        section: Section {
            glop_glyf: 386,
            general: FlattenedGeneral { olle_bolle: 42.0 },
            renamed: RenamedSection { snop_snyf: 42 },
        },
    };

    assert_eq!(loaded, expected);
}

const TEST_INI: &str = r#"
argle=Hello, world!
Bargle=17
olle_bolle=84.0

[Section]
glop_glyf=123

[SomethingElse]
snop_snyf=-42
"#;
