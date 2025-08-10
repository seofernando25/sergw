use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::time::{Duration, Instant};

use anyhow::Result;
use crossbeam_channel::Receiver;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};

use crate::state::SharedState;
use crate::metrics::ThroughputAverager;

#[derive(Default)]
pub struct Counters {
    pub bytes_in: AtomicU64,
    pub bytes_out: AtomicU64,
}

pub fn run_tui(
    shared: Arc<SharedState>,
    counters: Arc<Counters>,
    events: Receiver<String>,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut logs: Vec<String> = Vec::new();
    let mut last_in = 0u64;
    let mut last_out = 0u64;
    let mut avg_in = ThroughputAverager::new(5.0);
    let mut avg_out = ThroughputAverager::new(5.0);
    let mut last_time = Instant::now();

    while !stop.load(Ordering::Relaxed) {
        while let Ok(ev) = events.try_recv() {
            logs.push(ev);
            if logs.len() > 100 {
                logs.remove(0);
            }
        }

        let now = Instant::now();
        let dt = now.duration_since(last_time).as_secs_f64().max(0.001);
        let bi = counters.bytes_in.load(Ordering::Relaxed);
        let bo = counters.bytes_out.load(Ordering::Relaxed);
        let tin = avg_out.update(bi - last_in, dt) as u64;   // TCP -> serial (outbound, smoothed)
        let tout = avg_in.update(bo - last_out, dt) as u64; // serial -> TCP (inbound, smoothed)
        last_in = bi;
        last_out = bo;
        last_time = now;

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(5), Constraint::Length(4), Constraint::Min(0)].as_ref())
                .split(f.size());

            // Active connections
            let items: Vec<ListItem> = shared
                .tcp_connections
                .iter()
                .map(|e| ListItem::new(e.key().to_string()))
                .collect();
            let list = List::new(items).block(Block::default().title("Connections").borders(Borders::ALL));
            f.render_widget(list, chunks[0]);

            // Throughput
            // Show inbound (from serial) and outbound (to serial)
            let throughput = Paragraph::new(format!("Inbound: {tout} B/s\nOutbound: {tin} B/s"))
                .block(Block::default().title("Throughput").borders(Borders::ALL));
            f.render_widget(throughput, chunks[1]);

            // Logs (auto-scroll: show last lines that fit, top-to-bottom)
            let viewport = chunks[2].height.saturating_sub(2) as usize; // minus borders
            let start = logs.len().saturating_sub(viewport);
            let log_items: Vec<ListItem> = logs.iter().skip(start).map(|l| ListItem::new(l.clone())).collect();
            let log_list = List::new(log_items).block(Block::default().title("Events").borders(Borders::ALL));
            f.render_widget(log_list, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q')
                    || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    stop.store(true, Ordering::Relaxed);
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

