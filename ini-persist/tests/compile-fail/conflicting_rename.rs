use ini_persist::load::LoadProperty;

#[derive(LoadProperty)]
struct Foo {
    #[ini(rename = "argle")]
    #[ini(rename = "bargle")]
    bar: u8,
}

fn main() {
}
