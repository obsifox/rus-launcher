use include_dir::{include_dir, Dir};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;
use std::time::Duration;

static LAUNCHER: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets/launcher");

fn mime(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("ttf") => "font/ttf",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("mp4") => "video/mp4",
        _ => "application/octet-stream",
    }
}

fn clean(request: &str) -> String {
    let path = request.split_whitespace().nth(1).unwrap_or("/");
    let path = path.split('?').next().unwrap_or(path);
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return "index.html".to_string();
    }
    trimmed.replace("..", "")
}

fn send(stream: &mut TcpStream, status: &str, body: &[u8], kind: &str) {
    let head = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        status,
        kind,
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn serve(listener: TcpListener) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));

        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
            continue;
        }
        let path = clean(&line);

        let mut blank = String::new();
        while let Ok(size) = reader.read_line(&mut blank) {
            if size == 0 || size <= 2 {
                break;
            }
            blank.clear();
        }

        match LAUNCHER.get_file(&path) {
            Some(file) => send(&mut stream, "200 OK", file.contents(), mime(&path)),
            None => match LAUNCHER.get_file("index.html") {
                Some(file) => {
                    send(&mut stream, "200 OK", file.contents(), "text/html; charset=utf-8")
                }
                None => send(&mut stream, "404 Not Found", b"not found", "text/plain"),
            },
        }
        thread::spawn(move || drop(stream));
    }
}

fn open(url: &str) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        for browser in ["xdg-open", "gio", "sensible-browser"] {
            if std::process::Command::new(browser)
                .arg(url)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .is_ok()
            {
                return;
            }
        }
        println!("Rus Launcher is running at {}", url);
    }
}

/// Opens the interface in a window of its own. Stays here until the window is closed.
#[cfg(feature = "native")]
fn run_window(url: &str) -> Result<(), String> {
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    };
    use wry::WebViewBuilder;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Rus Launcher")
        .with_inner_size(tao::dpi::LogicalSize::new(1360.0, 860.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(1100.0, 680.0))
        .build(&event_loop)
        .map_err(|e| e.to_string())?;

    let builder = WebViewBuilder::new().with_url(url);

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "android"))]
    let _webview = builder.build(&window).map_err(|e| e.to_string())?;

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "android")))]
    let _webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().ok_or("no webview container")?;
        builder.build_gtk(vbox).map_err(|e| e.to_string())?
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
    let port = listener.local_addr().expect("port").port();
    let url = format!("http://127.0.0.1:{}/", port);
    println!("Rus Launcher on {}", url);

    thread::spawn(move || serve(listener));

    #[cfg(feature = "native")]
    {
        if let Err(reason) = run_window(&url) {
            println!("No window: {}. Opening the browser instead.", reason);
            open(&url);
        }
    }
    #[cfg(not(feature = "native"))]
    open(&url);

    // keep the server up while the window, or the browser tab, is open
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}
