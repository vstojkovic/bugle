use ini_persist::Property;

#[derive(Property)]
#[ini(repr)]
#[repr(C)]
enum Foo {
    Arglebargle,
    GlopGlyf,
}

fn main() {
}
