use std::{
    env,
    fs::File,
    io::{Error, ErrorKind, Read, Result, Write},
    net::{TcpListener, TcpStream},
    path::Path,
};

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221")?;

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                if let Err(e) = handle_connection(_stream) {
                    eprintln!("Error handling connection: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<()> {
    let mut buffer = [0; 1024];
    let mut content_length = 0;

    loop {
        let bytes_read = stream.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
        if request_str.contains("\r\n\r\n") {
            let lines: Vec<&str> = request_str.split("\r\n").collect();
            for line in lines.iter() {
                if line.starts_with("Content-Length: ") {
                    content_length = line
                        .splitn(2, ": ")
                        .nth(1)
                        .unwrap()
                        .trim()
                        .parse::<usize>()
                        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
                    break;
                }
            }
            break;
        }
    }

    let request = String::from_utf8_lossy(&buffer[..]);
    let tokens: Vec<&str> = request
        .lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .collect();

    if tokens.len() < 2 {
        stream.write(b"HTTP/1.1 400 Bad Request\r\n\r\n")?;
        return Ok(());
    }

    match (tokens[0], tokens[1]) {
        ("GET", path) if path.starts_with("/echo") => {
            let echo_path = path.trim_start_matches("/echo/");

            stream.write(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    echo_path.len(),
                    echo_path
                )
                .as_bytes(),
            )?;
        }
        ("GET", path) if path.starts_with("/files") => {
            if let Some(dir) = env::args().nth(2) {
                let filename = path.trim_start_matches("/files/");
                let file_path = Path::new(&dir).join(filename);

                if let Ok(mut file) = File::open(file_path) {
                    let mut buf = Vec::new();
                    file.read_to_end(&mut buf)?;

                    stream.write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
                            buf.len()
                        ).as_bytes()
                    )?;
                    stream.write_all(&buf)?;
                } else {
                    stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n")?;
                }
            }
        }
        ("POST", path) if path.starts_with("/files") => {
            let content = request.lines().last().unwrap_or("");

            if let Some(directory) = env::args().nth(2) {
                let filename = path.trim_start_matches("/files");
                let mut file = File::create(directory.to_owned() + filename)?;

                file.write_all(&content.as_bytes()[..content_length])?;

                stream.write_all(b"HTTP/1.1 201 Created\r\n\r\n")?;
            } else {
                stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n")?;
            }
        }
        ("GET", "/user-agent") => {
            let user_agent = request
                .lines()
                .find(|line| line.starts_with("User-Agent: "))
                .map(|line| line.splitn(2, ": ").nth(1).unwrap_or(""))
                .unwrap_or("");

            stream.write(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    user_agent.len(),
                    user_agent
                )
                .as_bytes(),
            )?;
        }
        ("GET", "/") => {
            stream.write(format!("HTTP/1.1 200 OK\r\n\r\n\r\n\r\n").as_bytes())?;
        }
        _ => {
            stream.write(b"HTTP/1.1 404 Not Found\r\n\r\n")?;
        }
    }
    Ok(())
}
