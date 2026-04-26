use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};

#[derive(Debug)]
struct RequestLine<'a> {
    method: &'a str,
    path: &'a str,
    version: &'a str,
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878")?;

    println!("Listening on http://127.0.0.1:7878");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_connection(stream) {
                    eprintln!("Request failed: {error}");
                }
            }
            Err(error) => eprintln!("Connection failed: {error}"),
        }
    }

    Ok({})
}

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);

    match parse_request_line(&request) {
        Some(RequestLine {
            method: "GET",
            path: "/",
            ..
        }) => serve_file(&mut stream, "public/index.html"),
        Some(RequestLine {
            method: "GET",
            path: "/about",
            ..
        }) => serve_file(&mut stream, "public/about.html"),
        Some(RequestLine {
            method: "GET",
            path: "/health",
            ..
        }) => send_response(&mut stream, "200 OK", "text/plain", b"ok\n"),
        Some(RequestLine {
            method: "GET",
            ..
        }) => send_response(&mut stream, "404 Not Found", "text/plain", b"Not Found\n"),
        Some(_) => send_response(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            b"Method not allowed\n",
        ),
        None => send_response(&mut stream, "400 Bad Request", "text/plain", b"Bad Request\n"),
    }
}

fn parse_request_line(request: &str) -> Option<RequestLine<'_>> {
    let first_line = request.lines().next()?;
    let mut parts = first_line.split_whitespace();

    Some(RequestLine { 
        method: parts.next()?, 
        path: parts.next()?, 
        version: parts.next()?,
    })
}

fn serve_file(stream: &mut TcpStream, path: &str) -> io::Result<()> {
    match fs::read(path) {
        Ok(contents) => send_response(stream, "200 OK", "text/html", &contents),
        Err(_) => send_response(
            stream,
            "500 Internal Server Error",
            "text/plain",
            b"Could not read file\n",
        ),
    }
}

fn send_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> io::Result<()> {
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: {content_type}\r\nConnection: close\r\n\r\n",
        body.len()
    );

    stream.write_all(headers.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}
