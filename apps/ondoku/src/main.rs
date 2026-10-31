//! `sekka-ondoku`: the reading buddy.

use sekka_kid::Store;
use sekka_kid_kit::instance::{self, Claim};
use sekka_ondoku::Ondoku;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "sekka-ondoku\n\nOndoku, a reading buddy: kids read aloud to Fumi the owl, from kindergarten to\n5th grade. Kids' profiles, progress and time are kept in {}.\nWhat the microphone hears is never kept.",
            Store::default_root().display()
        );
        return;
    }
    let mut app = Ondoku::new(Store::open(), false);
    match instance::claim("ondoku", &args) {
        Claim::Handed => return,
        Claim::First(listener) => app.instance = Some(listener),
        Claim::Alone => {}
    }
    aizome_kit::run(app);
}
