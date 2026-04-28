# Building a Basic HTTP Server in Rust

This guide walks through building a small HTTP/1.1 server with Rust using only the standard library. The goal is to learn low-level programming concepts: sockets, byte buffers, parsing text protocols, file I/O, and explicit error handling.

By the end, you will have a server that can:

- Listen for TCP connections.
- Read raw HTTP requests.
- Parse the request line.
- Send valid HTTP responses.
- Serve a few static files.
- Return useful error responses.

## 1. Install Rust

Rust is installed with `rustup`, the official Rust toolchain manager.

### macOS or Linux

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, restart your terminal or run:

```sh
source "$HOME/.cargo/env"
```

### Windows

Install Rust from:

```text
https://rustup.rs
```

Follow the installer prompts. You may also need the Microsoft C++ Build Tools if the installer asks for them.

### Verify the installation

```sh
rustc --version
cargo --version
```

`rustc` is the Rust compiler. `cargo` is Rust's build tool, dependency manager, and test runner.

## 2. Create a New Rust Project

Create a binary application:

```sh
cargo new rust_http_server
cd rust_http_server
```

Cargo creates this structure:

```text
rust_http_server/
  Cargo.toml
  src/
    main.rs
```

Run the starter app:

```sh
cargo run
```

You should see:

```text
Hello, world!
```

## 3. Understand the HTTP Exchange

HTTP runs on top of TCP. A browser opens a TCP connection to a server, sends bytes that look like text, and waits for bytes back.

A simple HTTP request looks like this:

```http
GET / HTTP/1.1
Host: localhost:7878
User-Agent: curl/8.0

```

A simple HTTP response looks like this:

```http
HTTP/1.1 200 OK
Content-Length: 13
Content-Type: text/plain

Hello, world!
```

Important details:

- The first request line contains the method, path, and HTTP version.
- Headers are key-value metadata.
- A blank line separates headers from the optional body.
- HTTP lines end with `\r\n`, not just `\n`.

## 4. Listen for TCP Connections

Replace `src/main.rs` with:

```rust
use std::io;
use std::net::TcpListener;

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878")?;

    println!("Listening on http://127.0.0.1:7878");

    for stream in listener.incoming() {
        match stream {
            Ok(_) => println!("Received a connection"),
            Err(error) => eprintln!("Connection failed: {error}"),
        }
    }

    Ok(())
}
```

Run it:

```sh
cargo run
```

In another terminal, connect with `curl`:

```sh
curl http://127.0.0.1:7878
```

The client will hang because the server accepts the connection but does not send a response yet. Stop the server with `Ctrl+C`.

What this teaches:

- `TcpListener::bind` opens a socket.
- `127.0.0.1` means localhost.
- `7878` is the port.
- `incoming()` yields TCP streams from clients.
- `?` returns early if binding fails.

## 5. Read Raw Request Bytes

Now read from each TCP stream.

```rust
use std::io::{self, Read};
use std::net::{TcpListener, TcpStream};

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878")?;

    println!("Listening on http://127.0.0.1:7878");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_connection(stream)?,
            Err(error) => eprintln!("Connection failed: {error}"),
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer)?;

    println!("Read {bytes_read} bytes");
    println!("{}", String::from_utf8_lossy(&buffer[..bytes_read]));

    Ok(())
}
```

Run the server and request it:

```sh
curl http://127.0.0.1:7878
```

You should see the raw HTTP request printed in the server terminal.

What this teaches:

- TCP data arrives as bytes, not strings.
- A fixed-size buffer is common in low-level network programming.
- `read` returns how many bytes were actually read.
- `String::from_utf8_lossy` lets you inspect mostly-text data safely.

## 6. Send a Basic HTTP Response

The client needs a valid HTTP response. Add `Write` and send bytes back.

```rust
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878")?;

    println!("Listening on http://127.0.0.1:7878");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_connection(stream)?,
            Err(error) => eprintln!("Connection failed: {error}"),
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer)?;

    println!("{}", String::from_utf8_lossy(&buffer[..bytes_read]));

    let body = "Hello from Rust!";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
        body.len(),
        body
    );

    stream.write_all(response.as_bytes())?;
    stream.flush()?;

    Ok(())
}
```

Try it:

```sh
curl -i http://127.0.0.1:7878
```

The `-i` flag tells `curl` to show response headers.

What this teaches:

- HTTP responses are just bytes written to a TCP stream.
- `Content-Length` tells the client how many bytes are in the body.
- `write_all` keeps writing until the full response is sent.
- `flush` asks the stream to send buffered data.

## 7. Parse the Request Line

Now inspect the method and path.

```rust
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
            Ok(stream) => handle_connection(stream)?,
            Err(error) => eprintln!("Connection failed: {error}"),
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);

    let request_line = parse_request_line(&request);

    let body = match request_line {
        Some(line) => format!(
            "Method: {}\nPath: {}\nVersion: {}\n",
            line.method, line.path, line.version
        ),
        None => "Could not parse request line\n".to_string(),
    };

    send_response(&mut stream, "200 OK", "text/plain", &body)
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

fn send_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &str,
) -> io::Result<()> {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: {content_type}\r\n\r\n{body}",
        body.len()
    );

    stream.write_all(response.as_bytes())?;
    stream.flush()
}
```

Try different paths:

```sh
curl http://127.0.0.1:7878/
curl http://127.0.0.1:7878/about
```

What this teaches:

- Rust string slices can borrow from an existing request string.
- `Option` is useful when parsing can fail.
- `split_whitespace` handles spaces between request-line parts.
- Small helper functions keep networking and response formatting separate.

## 8. Route Requests

Return different responses based on the requested path.

Update `handle_connection`:

```rust
fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);

    match parse_request_line(&request) {
        Some(RequestLine {
            method: "GET",
            path: "/",
            ..
        }) => send_response(&mut stream, "200 OK", "text/html", "<h1>Home</h1>"),

        Some(RequestLine {
            method: "GET",
            path: "/health",
            ..
        }) => send_response(&mut stream, "200 OK", "text/plain", "ok\n"),

        Some(RequestLine { method: "GET", .. }) => {
            send_response(&mut stream, "404 Not Found", "text/plain", "Not found\n")
        }

        Some(_) => send_response(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            "Method not allowed\n",
        ),

        None => send_response(
            &mut stream,
            "400 Bad Request",
            "text/plain",
            "Bad request\n",
        ),
    }
}
```

Try:

```sh
curl -i http://127.0.0.1:7878/
curl -i http://127.0.0.1:7878/health
curl -i http://127.0.0.1:7878/missing
curl -i -X POST http://127.0.0.1:7878/
```

What this teaches:

- Pattern matching can route requests clearly.
- HTTP status codes communicate outcome.
- `400`, `404`, and `405` mean different kinds of failure.

## 9. Serve Static HTML Files

Create a `public` directory:

```sh
mkdir public
```

Create `public/index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>Rust HTTP Server</title>
  </head>
  <body>
    <h1>Hello from a static file</h1>
    <p>This page was served by your Rust server.</p>
  </body>
</html>
```

Create `public/about.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <title>About</title>
  </head>
  <body>
    <h1>About</h1>
    <p>This server uses TcpListener and TcpStream directly.</p>
  </body>
</html>
```

Now update `src/main.rs`:

```rust
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

    Ok(())
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

        Some(RequestLine { method: "GET", .. }) => {
            send_response(&mut stream, "404 Not Found", "text/plain", b"Not found\n")
        }

        Some(_) => send_response(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            b"Method not allowed\n",
        ),

        None => send_response(
            &mut stream,
            "400 Bad Request",
            "text/plain",
            b"Bad request\n",
        ),
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
```

Try it in a browser:

```text
http://127.0.0.1:7878/
http://127.0.0.1:7878/about
```

What this teaches:

- Static files are read as bytes with `fs::read`.
- Response bodies do not have to be valid UTF-8 strings.
- `Connection: close` tells the client the server will close the TCP connection after the response.
- Per-request errors should not crash the whole server.

## 10. Add Simple MIME Types

HTML is not the only kind of static file. Add a helper:

```rust
fn content_type_for_path(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "application/octet-stream"
    }
}
```

Then update `serve_file`:

```rust
fn serve_file(stream: &mut TcpStream, path: &str) -> io::Result<()> {
    match fs::read(path) {
        Ok(contents) => {
            let content_type = content_type_for_path(path);
            send_response(stream, "200 OK", content_type, &contents)
        }
        Err(_) => send_response(
            stream,
            "500 Internal Server Error",
            "text/plain",
            b"Could not read file\n",
        ),
    }
}
```

What this teaches:

- Browsers use `Content-Type` to decide how to interpret bytes.
- `application/octet-stream` means generic binary data.
- A small manual MIME mapper is fine for learning, while production servers use more complete libraries.

## 11. Build a Safer Static File Router

It is dangerous to directly map request paths to filesystem paths. A request like `/../../secret.txt` should never escape the public directory.

For this beginner server, keep an explicit route table:

```rust
fn file_for_route(path: &str) -> Option<&'static str> {
    match path {
        "/" => Some("public/index.html"),
        "/about" => Some("public/about.html"),
        _ => None,
    }
}
```

Then simplify the GET routing:

```rust
Some(RequestLine {
    method: "GET",
    path,
    ..
}) => match file_for_route(path) {
    Some(file_path) => serve_file(&mut stream, file_path),
    None if path == "/health" => {
        send_response(&mut stream, "200 OK", "text/plain", b"ok\n")
    }
    None => send_response(&mut stream, "404 Not Found", "text/plain", b"Not found\n"),
},
```

What this teaches:

- Low-level servers must think carefully about filesystem boundaries.
- Explicit routing avoids path traversal bugs.
- Security is easier when the allowed behavior is small and clear.

## 12. Final Version

Here is a complete `src/main.rs`:

```rust
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

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);

    match parse_request_line(&request) {
        Some(RequestLine {
            method: "GET",
            path,
            ..
        }) => match file_for_route(path) {
            Some(file_path) => serve_file(&mut stream, file_path),
            None if path == "/health" => {
                send_response(&mut stream, "200 OK", "text/plain", b"ok\n")
            }
            None => send_response(&mut stream, "404 Not Found", "text/plain", b"Not found\n"),
        },

        Some(_) => send_response(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            b"Method not allowed\n",
        ),

        None => send_response(
            &mut stream,
            "400 Bad Request",
            "text/plain",
            b"Bad request\n",
        ),
    }
}

fn parse_request_line(request: &str) -> Option<RequestLine<'_>> {
    let first_line = request.lines().next()?;
    let mut parts = first_line.split_whitespace();

    let line = RequestLine {
        method: parts.next()?,
        path: parts.next()?,
        version: parts.next()?,
    };

    if parts.next().is_some() {
        return None;
    }

    Some(line)
}

fn file_for_route(path: &str) -> Option<&'static str> {
    match path {
        "/" => Some("public/index.html"),
        "/about" => Some("public/about.html"),
        _ => None,
    }
}

fn serve_file(stream: &mut TcpStream, path: &str) -> io::Result<()> {
    match fs::read(path) {
        Ok(contents) => {
            let content_type = content_type_for_path(path);
            send_response(stream, "200 OK", content_type, &contents)
        }
        Err(_) => send_response(
            stream,
            "500 Internal Server Error",
            "text/plain",
            b"Could not read file\n",
        ),
    }
}

fn content_type_for_path(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "application/octet-stream"
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
```

Run:

```sh
cargo run
```

Test:

```sh
curl -i http://127.0.0.1:7878/
curl -i http://127.0.0.1:7878/about
curl -i http://127.0.0.1:7878/health
curl -i http://127.0.0.1:7878/missing
curl -i -X POST http://127.0.0.1:7878/
```

## 13. Debugging Tips

### Port already in use

If you see an error like `Address already in use`, another process is using port `7878`. Stop the other server or change the port:

```rust
TcpListener::bind("127.0.0.1:8080")?;
```

### Browser keeps loading

Make sure your response includes:

```http
Content-Length: ...
Connection: close
```

Also make sure there is a blank line between headers and body:

```text
\r\n\r\n
```

### File route returns 500

Check that your current working directory contains the `public` directory:

```sh
pwd
ls public
```

Run the server from the project root with:

```sh
cargo run
```

## 14. Exercises

Try these once the basic server works:

- Add a `/contact` page.
- Add a CSS file and return `text/css`.
- Log the client address with `stream.peer_addr()`.
- Reject non-HTTP/1.1 requests with `505 HTTP Version Not Supported`.
- Increase the buffer size and test with larger headers.
- Add a tiny thread pool so multiple slow requests do not block each other.
- Parse headers into a `HashMap<String, String>`.

## 15. What to Learn Next

This guide intentionally avoids frameworks so the protocol and system calls stay visible. After this, learn:

- Ownership and borrowing in larger Rust programs.
- `Result`, `Option`, and custom error types.
- Threads with `std::thread`.
- Async Rust with `tokio`.
- HTTP libraries such as `hyper`.
- Production server concerns: TLS, timeouts, streaming bodies, request limits, and security hardening.

