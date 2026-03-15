use super::{
    log::{
        Log,
        command::{Command, Execute},
    },
    tui,
};
use crate::log::{DATA_PATH, MEMTABLE_PATH, SSTABLES_PATH};

/// Opens the log and runs the REPL until quit.
pub fn run() -> anyhow::Result<()> {
    tui::welcome();
    let mut log = Log::new(DATA_PATH, MEMTABLE_PATH, SSTABLES_PATH, false)?;
    loop {
        let command = Command::unfallible_get();
        if matches!(command, Command::Quit) {
            tui::hr();
            println!("Exiting process");
            break;
        }
        log.execute(command)?;
        tui::hr();
    }
    Ok(())
}
