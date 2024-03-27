use ini::Properties;
use ini_persist::{LoadProperty, Result};

#[derive(LoadProperty)]
struct Foo {
    #[ini(load_in_with = my_load_in)]
    #[ini(load_in_with = my_load_in)]
    argle: u8,

    #[ini(load_in_with = my_load_in)]
    #[ini(load_with = my_load)]
    bargle: u8,

    #[ini(load_in_with = my_load_in)]
    #[ini(parse_with = my_parse)]
    glop_glyf: u8,
}

fn my_load_in(field: &mut u8, section: &Properties, key: &str) -> Result<()> {
    todo!()
}

fn my_load(section: &Properties, key: &str) -> Result<Option<u8>> {
    todo!()
}

fn my_parse(value: &str) -> Result<u8> {
    todo!()
}

fn main() {
}
