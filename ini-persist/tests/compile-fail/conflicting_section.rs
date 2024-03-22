use ini_persist::IniLoad;

#[derive(IniLoad)]
#[ini(section = "This")]
#[ini(section = "That")]
struct Foo {
    argle: u8,
    bargle: String,
}

#[derive(IniLoad)]
#[ini(general)]
#[ini(section = "Specific")]
struct Bar {
    glop: u8,
    glyf: String,
}

#[derive(IniLoad)]
#[ini(section = "Specific")]
#[ini(general)]
struct Baz {
    olle: u8,
    bolle: String,
}

fn main() {
}
