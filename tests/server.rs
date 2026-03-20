use nighthawk::{log::Log, run::Runner};
use std::{
    io::{BufRead, BufReader, BufWriter, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};
use tempfile::tempdir;

/// Binds a server on a random port, spawns it in a background thread.
/// Returns the bound address so tests can connect to it.
fn start_server() -> std::net::SocketAddr {
    let dir = tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let wal_path = dir.path().join("wal");
    let sstables_path = dir.path().join("sstables");
    let mut log = Log::new(dir.path(), wal_path, sstables_path, true).unwrap();

    thread::spawn(move || {
        // Keep dir alive for the lifetime of the thread
        let _dir = dir;
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            let reader = BufReader::new(stream.try_clone().unwrap());
            let writer = BufWriter::new(stream);
            let mut runner = Runner::new(reader, writer);
            runner.run(&mut log).unwrap();
        }
    });

    addr
}

/// Sends a command line to the server and returns the response line (trimmed).
fn send(stream: &mut BufReader<TcpStream>, writer: &mut impl Write, cmd: &str) -> String {
    writeln!(writer, "{}", cmd).unwrap();
    writer.flush().unwrap();
    let mut response = String::new();
    stream.read_line(&mut response).unwrap();
    response.trim().to_string()
}

// --- SET ---

#[test]
fn set_returns_ok() {
    let addr = start_server();
    let stream = TcpStream::connect(addr).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = BufWriter::new(stream);

    let response = send(&mut reader, &mut writer, "SET a 1");
    // TODO: assert response equals the expected OK string
}

// --- GET ---

#[test]
fn get_existing_key_returns_value() {
    let addr = start_server();
    let stream = TcpStream::connect(addr).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = BufWriter::new(stream);

    send(&mut reader, &mut writer, "SET a 1");
    let response = send(&mut reader, &mut writer, "GET a");
    // TODO: assert response contains the value
}

#[test]
fn get_missing_key_returns_not_found() {
    let addr = start_server();
    let stream = TcpStream::connect(addr).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = BufWriter::new(stream);

    let response = send(&mut reader, &mut writer, "GET missing");
    // TODO: assert response equals the expected not-found string
}

// --- DEL ---

#[test]
fn del_existing_key_returns_ok() {
    let addr = start_server();
    let stream = TcpStream::connect(addr).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = BufWriter::new(stream);

    send(&mut reader, &mut writer, "SET a 1");
    let response = send(&mut reader, &mut writer, "DEL a");
    // TODO: assert response equals the expected deleted string
}

#[test]
fn del_missing_key_returns_not_found() {
    let addr = start_server();
    let stream = TcpStream::connect(addr).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = BufWriter::new(stream);

    let response = send(&mut reader, &mut writer, "DEL missing");
    // TODO: assert response equals the expected not-found string
}

// --- Error handling ---

#[test]
fn invalid_command_returns_err() {
    let addr = start_server();
    let stream = TcpStream::connect(addr).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = BufWriter::new(stream);

    let response = send(&mut reader, &mut writer, "INVALID");
    // TODO: assert response starts with "ERR"
}

// --- Sequencing ---

#[test]
fn set_then_del_then_get_returns_not_found() {
    let addr = start_server();
    let stream = TcpStream::connect(addr).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = BufWriter::new(stream);

    send(&mut reader, &mut writer, "SET a 1");
    send(&mut reader, &mut writer, "DEL a");
    let response = send(&mut reader, &mut writer, "GET a");
    // TODO: assert response equals the expected not-found string
}
