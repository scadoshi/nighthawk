use nighthawk::{
    log::{DATA_PATH, Log, SSTABLES_PATH, WAL_PATH},
    run::Runner,
};
use std::{
    io::{BufReader, BufWriter},
    net::TcpListener,
};

fn server() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let address = std::env::var("ADDRESS")?;
    let port = std::env::var("PORT")?;
    let bind_address = format!("{}:{}", address, port);
    let mut log = Log::new(DATA_PATH, WAL_PATH, SSTABLES_PATH, false)?;
    let listener = TcpListener::bind(bind_address)?;
    println!("Listening on {}", listener.local_addr()?);
    for stream in listener.incoming() {
        let stream = stream?;
        let reader = BufReader::new(stream.try_clone()?);
        let writer = BufWriter::new(stream);
        let mut runner = Runner::new(reader, writer);
        runner.run(&mut log)?;
    }
    Ok(())
}

fn main() {
    if let Err(e) = server() {
        eprintln!("{}", e);
    }
}
