use ini_persist::IniLoad;

#[derive(IniLoad)]
struct Foo {
    #[ini(key = "argle")]
    #[ini(key = "bargle")]
    bar: u8,
}

fn main() {
}
