use ini_persist::load::LoadProperty;

#[derive(LoadProperty)]
#[ini(repr)]
#[ini(ignore_case)]
enum Foo {
    Arglebargle,
    GlopGlyf,
}

#[derive(LoadProperty)]
#[ini(ignore_case)]
#[ini(repr)]
enum Bar {
    OlleBolle,
    SnopSnyf,
}

fn main() {
}
