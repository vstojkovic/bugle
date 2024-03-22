use ini_persist::{IniLoad, Result};

#[derive(IniLoad)]
struct Foo {
    argle_bargle: u8,
}

#[derive(IniLoad)]
struct Bar {
    #[ini(flatten)]
    #[ini(key = "Glop")]
    glop: Foo,

    #[ini(flatten)]
    #[ini(parse_with = my_parse)]
    glyf: Foo,

    #[ini(key = "Glop")]
    #[ini(flatten)]
    olle_bolle: Foo,

    #[ini(parse_with = my_parse)]
    #[ini(flatten)]
    snop_snyf: Foo,
}

fn my_parse(value: &str) -> Result<Foo> {
    todo!()
}

fn main() {
}
