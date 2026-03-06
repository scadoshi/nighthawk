mod command;
mod index;
mod run;
mod tui;

fn main() {
    if let Err(e) = run::run() {
        eprintln!("{}", e);
    }
}
