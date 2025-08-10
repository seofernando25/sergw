use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossbeam_channel as channel;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};

use crate::cli::Chat;
use crate::metrics::ThroughputAverager;

pub fn run_chat(chat: Chat) -> Result<()> {
    // Connect TCP
    let mut stream = TcpStream::connect(chat.host)
        .with_context(|| format!("Connecting to {}", chat.host))?;
    stream.set_nodelay(true).ok();
    stream.set_nonblocking(true).ok();

    // UI setup
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let stop = Arc::new(AtomicBool::new(false));
    let rx_bytes = Arc::new(AtomicU64::new(0));
    let tx_bytes = Arc::new(AtomicU64::new(0));
    let (log_tx, log_rx) = channel::unbounded::<String>();

    // Reader thread
    let stop_r = stop.clone();
    let rx_b = rx_bytes.clone();
    let mut rstream = stream.try_clone()?;
    let log_tx_reader = log_tx.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        while !stop_r.load(Ordering::Relaxed) {
            match rstream.read(&mut buf) {
                Ok(0) => std::thread::sleep(Duration::from_millis(20)),
                Ok(n) => {
                    rx_b.fetch_add(n as u64, Ordering::Relaxed);
                    let s = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = log_tx_reader.send(format!("< {s}"));
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(_) => break,
            }
        }
    });

    let mut logs: Vec<String> = Vec::new();
    let mut input = String::new();
    let mut last_rx = 0u64;
    let mut last_tx = 0u64;
    let mut avg_in = ThroughputAverager::new(5.0);
    let mut avg_out = ThroughputAverager::new(5.0);
    let mut last_time = Instant::now();

    loop {
        while let Ok(line) = log_rx.try_recv() {
            logs.push(line);
            if logs.len() > 200 { logs.remove(0); }
        }

        // Throughput calc
        let now = Instant::now();
        let dt = now.duration_since(last_time).as_secs_f64().max(0.001);
        let rx = rx_bytes.load(Ordering::Relaxed);
        let tx = tx_bytes.load(Ordering::Relaxed);
        let inbound = avg_in.update(rx - last_rx, dt) as u64;   // from TCP (smoothed)
        let outbound = avg_out.update(tx - last_tx, dt) as u64; // to TCP (smoothed)
        last_rx = rx; last_tx = tx; last_time = now;

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(3),
                    Constraint::Length(3),
                ])
                .split(f.size());

            let header = Paragraph::new(format!("chat | {} | In: {} B/s Out: {} B/s", chat.host, inbound, outbound));
            f.render_widget(header, chunks[0]);

            // Auto-scroll: render only the last lines that fit
            let viewport = chunks[1].height.saturating_sub(2) as usize; // minus borders
            let start = logs.len().saturating_sub(viewport);
            let items: Vec<ListItem> = logs.iter().skip(start).map(|l| ListItem::new(l.clone())).collect();
            let list = List::new(items).block(Block::default().title("Messages").borders(Borders::ALL));
            f.render_widget(list, chunks[1]);

            let input_box = Paragraph::new(input.clone())
                .block(Block::default().title("Input (Enter to send, Ctrl+C to quit)").borders(Borders::ALL));
            f.render_widget(input_box, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(k) = event::read()? {
                match k.code {
                    KeyCode::Char('c') if k.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => break,
                    KeyCode::Char(c) => input.push(c),
                    KeyCode::Backspace => { input.pop(); },
                    KeyCode::Enter => {
                        if !input.is_empty() {
                            let mut to_send = input.clone();
                            to_send.push('\n');
                            if let Err(e) = stream.write_all(to_send.as_bytes()) {
                                let _ = log_tx.send(format!("! write error: {e}"));
                            } else {
                                tx_bytes.fetch_add(to_send.len() as u64, Ordering::Relaxed);
                                let _ = log_tx.send(format!("> {}", input));
                            }
                            input.clear();
                        }
                    }
                    KeyCode::Esc => input.clear(),
                    _ => {}
                }
            }
        }
    }

    stop.store(true, Ordering::Relaxed);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}


