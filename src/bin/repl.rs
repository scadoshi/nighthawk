use nighthawk::{
    log::{DATA_PATH, Log, SSTABLES_PATH, WAL_PATH},
    run::Runner,
    tui,
};
use std::io::{BufReader, stdin, stdout};

fn repl() -> anyhow::Result<()> {
    tui::welcome();
    let mut log = Log::new(DATA_PATH, WAL_PATH, SSTABLES_PATH, false)?;
    let mut runner = Runner::new(BufReader::new(stdin().lock()), stdout().lock());
    runner.run(&mut log)
}

fn main() {
    if let Err(e) = repl() {
        eprintln!("{}", e);
    }
}
