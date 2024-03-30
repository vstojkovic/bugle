use ini_persist::load::LoadProperty;

#[derive(LoadProperty)]
#[ini(repr)]
#[repr(C)]
enum Foo {
    Arglebargle,
    GlopGlyf,
}

fn main() {
}
