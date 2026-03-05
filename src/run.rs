use crate::{command::Command, tui};
use std::{collections::HashMap, fs::OpenOptions, io::Read};

pub fn run() -> anyhow::Result<()> {
    tui::welcome();

    let mut buf = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open("data.log")?;
    let mut data = Vec::<u8>::new();
    buf.read_to_end(&mut data)?;

    let mut index = HashMap::<String, u64>::new();
    let mut pos: u64 = 0;

    while (pos as usize) < data.len() {
        let slice = &data[pos as usize..];
        match wincode::deserialize::<(String, String)>(slice) {
            Ok((k, v)) => {
                let size = wincode::serialized_size(&(k.clone(), v))?;
                index.insert(k, pos);
                pos += size;
            }
            Err(_) => break,
        }
    }

    loop {
        Command::unfallible_get().execute(&mut buf, &mut index)?;
        tui::hr();
        if false {
            break;
        }
    }

    Ok(())
}
