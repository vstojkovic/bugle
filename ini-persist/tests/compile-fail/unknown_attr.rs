use ini_persist::IniLoad;

#[derive(IniLoad)]
#[ini(bogus)]
struct Foo {
    argle: u8,
    bargle: String,
}

fn main() {
}
