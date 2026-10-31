//! `sekka-kids`: kids' profiles and reports, for grown-ups.

use sekka_kid::Store;
use sekka_kid_kit::instance::{self, Claim};
use sekka_kids::Kids;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "sekka-kids\n\nSekka Kids: kids' profiles, daily time and reports across Sekka Edu, kept in\n{}.",
            Store::default_root().display()
        );
        return;
    }
    let mut app = Kids::new(Store::open());
    match instance::claim("kids", &args) {
        Claim::Handed => return,
        Claim::First(listener) => app.instance = Some(listener),
        Claim::Alone => {}
    }
    aizome_kit::run(app);
}
