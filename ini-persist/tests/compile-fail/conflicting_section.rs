use ini_persist::load::{IniLoad, LoadProperty};

#[derive(IniLoad)]
struct Foo {
    #[ini(section = "This")]
    #[ini(section = "That")]
    argle: Section,
}

#[derive(IniLoad)]
struct Bar {
    #[ini(general)]
    #[ini(section = "Specific")]
    bargle: Section,
}

#[derive(IniLoad)]
struct Baz {
    #[ini(section = "Specific")]
    #[ini(general)]
    glop: Section,
}

#[derive(LoadProperty)]
struct Section {
    glyf: u8,
}

fn main() {
}
