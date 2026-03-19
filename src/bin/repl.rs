fn main() {
    if let Err(e) = nighthawk::run::run() {
        eprintln!("{}", e);
    }
}
