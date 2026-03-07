use super::{
    log::{
        Log,
        command::{Command, Execute},
    },
    tui,
};

pub fn run() -> anyhow::Result<()> {
    tui::welcome();
    let mut log = Log::new()?;
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
