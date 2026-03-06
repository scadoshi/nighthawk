use crate::{
    command::Command,
    index::{Index, IndexOps},
    tui,
};
use std::fs::OpenOptions;

pub const DATA_PATH: &str = "data.log";

pub fn run() -> anyhow::Result<()> {
    tui::welcome();

    let mut buf = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(DATA_PATH)?;

    let mut index = Index::from_buf(&mut buf)?;

    loop {
        let command = Command::unfallible_get();
        command.execute(&mut buf, &mut index)?;
        tui::hr();
        if false {
            break;
        }
    }

    Ok(())
}
