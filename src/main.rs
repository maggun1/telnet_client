use std::io::{self, BufRead, Write, Read, stdin};
use std::net::{TcpStream, ToSocketAddrs};
use std::thread;
use std::time::Duration;
use std::io::ErrorKind;
use std::process::exit;
use clap::{Arg, Command};

fn parse_timeout(timeout: &str) -> Duration {
    if timeout.ends_with("s") {
        Duration::from_secs(timeout[..timeout.len() - 1].parse::<u64>().unwrap())
    }
    else {
        eprintln!("Invalid timeout syntax: {}", timeout);
        eprintln!("Must consist of numbers and end in 's'.");
        eprintln!("Using default timeout of 10s.");

        Duration::from_secs(10)
    }
}

fn main() {
    let matches = Command::new("telnet_client")
        .arg(
            Arg::new("host")
                .required(true)
                .index(1))
        .arg(
            Arg::new("port")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .num_args(1)
                .default_value("10s"),
        )
        .get_matches();

    let host = matches.get_one::<String>("host").unwrap();
    let port = matches.get_one::<String>("port").unwrap();
    let timeout = parse_timeout(matches.get_one::<String>("timeout").unwrap());

    let addr =
        format!("{}:{}", host, port)
        .to_socket_addrs().
            unwrap()
            .next()
            .unwrap();

    let mut stream = TcpStream::connect_timeout(&addr, timeout).unwrap();
    let mut stream_clone = stream.try_clone().unwrap();

    let initial_message = "GET HTTP/1.0 /\r\n";
    stream.write_all(initial_message.as_bytes()).unwrap();

    let reader_thread = thread::spawn(move || {
        let mut buffer = [0; 512];
        loop {
            match stream_clone.read(&mut buffer) {
                Ok(0) => {
                    println!("Connection closed by server.");
                    exit(0);
                }
                Ok(n) => {
                    io::stdout().write_all(&buffer[..n]).unwrap();
                    io::stdout().flush().unwrap();
                }
                Err(ref e) if e.kind() == ErrorKind::ConnectionReset => {
                    eprintln!("Connection was reset by the server.");
                    exit(0);
                }
                Err(ref e) => {
                    eprintln!("Error reading from socket: {}", e);
                    exit(0);
                }
            }

        }
    });

    let writer_thread = thread::spawn(move || {
        let stdin = stdin();
        let mut handle = stdin.lock();
        let mut input_buffer = String::new();
        loop {
            match handle.read_line(&mut input_buffer) {
                Ok(0) => {
                    break;
                }
                Ok(_) => {
                    stream.write_all(input_buffer.as_bytes()).unwrap();
                    input_buffer.clear();
                }
                Err(e)  => {
                    eprintln!("Error reading from stdin: {}", e);
                    break;
                }
            }

        }

        println!("Input finished, shutting down socket.");
        if let Err(e) = stream.shutdown(std::net::Shutdown::Both) {
            eprintln!("Error shutting down socket: {}", e);
        }
    });

    reader_thread.join().unwrap();
    writer_thread.join().unwrap();
}
