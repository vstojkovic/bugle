use ini_persist::{IniLoad, LoadProperty};

#[derive(IniLoad)]
#[ini(bogus)]
struct Argle {
    argle: u8,
    bargle: String,
}

#[derive(IniLoad)]
struct Bargle {
    #[ini(bogus)]
    bargle: String,
}

#[derive(LoadProperty)]
#[ini(bogus)]
struct Glop {
    glop: i16,
}

#[derive(LoadProperty)]
struct Glyf {
    #[ini(bogus)]
    glyf: Option<bool>,
}

#[derive(LoadProperty)]
#[ini(bogus)]
enum Olle {
    Bolle,
}

#[derive(LoadProperty)]
enum Snop {
    #[ini(bogus)]
    Snyf,
}

fn main() {
}
