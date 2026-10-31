//! `sekka-anzan`: the mental abacus.

use sekka_anzan::Anzan;
use sekka_kid::Store;
use sekka_kid_kit::instance::{self, Claim};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "sekka-anzan\n\nAnzan, the mental abacus, for kindergarten and first grade. Kids' profiles,\nprogress and time are kept in {}.",
            Store::default_root().display()
        );
        return;
    }
    let mut app = Anzan::new(Store::open(), false);
    match instance::claim("anzan", &args) {
        Claim::Handed => return,
        Claim::First(listener) => app.instance = Some(listener),
        Claim::Alone => {}
    }
    aizome_kit::run(app);
}
