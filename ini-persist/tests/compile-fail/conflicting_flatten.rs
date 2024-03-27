use ini_persist::{LoadProperty, Result};

#[derive(LoadProperty)]
struct Foo {
    argle: u8,
}

#[derive(LoadProperty)]
struct Bar {
    #[ini(flatten)]
    #[ini(rename = "Bargle")]
    bargle: Foo,

    #[ini(flatten)]
    #[ini(key_format = "{name}{prefix}")]
    glop: Foo,

    #[ini(flatten)]
    #[ini(parse_with = my_parse)]
    glyf: Foo,

    #[ini(rename = "OlleBolle")]
    #[ini(flatten)]
    olle_bolle: Foo,

    #[ini(key_format = "{name}{prefix}")]
    #[ini(flatten)]
    snop: Foo,

    #[ini(parse_with = my_parse)]
    #[ini(flatten)]
    snyf: Foo,
}

fn my_parse(value: &str) -> Result<Foo> {
    todo!()
}

fn main() {
}
