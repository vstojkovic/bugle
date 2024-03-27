use ini_persist::LoadProperty;

#[derive(LoadProperty)]
#[ini(repr)]
#[repr(C)]
enum Foo {
    Arglebargle,
    GlopGlyf,
}

fn main() {
}
