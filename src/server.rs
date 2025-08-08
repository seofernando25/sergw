use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};
use bytes::Bytes;
use crossbeam_channel as channel;
use tracing::{error, info, warn};

use crate::cli::Listen;
use crate::serial::{configure_serial, select_serial_port};
use crate::state::SharedState;

pub fn run_listen(listen: Listen) -> Result<()> {
    let stop_flag = Arc::new(AtomicBool::new(false));
    {
        let stop = stop_flag.clone();
        let _ = ctrlc::set_handler(move || {
            stop.store(true, Ordering::Relaxed);
        });
    }

    let serial_path = select_serial_port(&listen.serial)?;
    info!(serial = %serial_path, baud = listen.baud, host = %listen.host, "Starting sergw");

    // Open serial once, clone for writer
    let serial_builder = serialport::new(&serial_path, listen.baud);
    let mut serial_port = configure_serial(serial_builder, &listen)
        .with_context(|| format!("Opening serial port {serial_path}"))?;
    let mut serial_writer_port = serial_port
        .try_clone()
        .with_context(|| format!("Cloning serial port {serial_path} for writer"))?;

    // Channels
    // - to_serial_rx: buffers from TCP -> serial writer
    let (to_serial_tx, to_serial_rx) = channel::bounded::<Bytes>(1024);

    // - shared state for broadcasting serial -> TCP
    let shared_state = Arc::new(Mutex::new(SharedState::new()));

    // Serial reader thread: serial -> broadcast
    let shared_state_for_reader = Arc::clone(&shared_state);
    let stop_reader = stop_flag.clone();
    let serial_reader = thread::spawn(move || -> Result<()> {
        let mut buffer = vec![0u8; 4096];
        while !stop_reader.load(Ordering::Relaxed) {
            match serial_port.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    let bytes = Bytes::copy_from_slice(&buffer[..n]);
                    shared_state_for_reader.lock().unwrap().broadcast(bytes);
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                    error!(?e, "Serial broken pipe");
                    break;
                }
                Err(e) => {
                    warn!(?e, "Error reading from serial");
                }
            }
        }
        Ok(())
    });

    // Serial writer thread: TCP -> serial
    let stop_writer = stop_flag.clone();
    let serial_writer = thread::spawn(move || -> Result<()> {
        while !stop_writer.load(Ordering::Relaxed) {
            match to_serial_rx.recv() {
                Ok(buf) => {
                    if let Err(e) = serial_writer_port.write_all(&buf) {
                        error!(?e, "Error writing to serial");
                        return Err(e.into());
                    }
                }
                Err(_e) => {
                    // Sender dropped; likely shutting down
                    break;
                }
            }
        }
        Ok(())
    });

    // TCP acceptor
    let listener = TcpListener::bind(listen.host)
        .with_context(|| format!("Binding TCP listener at {}", listen.host))?;
    listener
        .set_nonblocking(false)
        .context("Setting TCP listener blocking mode")?;

    loop {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }
        let (stream, addr) = match listener.accept() {
            Ok(conn) => conn,
            Err(e) => {
                warn!(?e, "Accept failed");
                continue;
            }
        };
        let mut stream_reader = stream.try_clone().context("Cloning TCP stream (reader)")?;
        let mut stream_writer = stream;
        let _ = stream_reader.set_nodelay(true);
        let _ = stream_writer.set_nodelay(true);
        info!(%addr, "Accepted connection");

        let to_serial_tx_conn = to_serial_tx.clone();
        let (to_tcp_tx, to_tcp_rx) = channel::bounded::<Bytes>(1024);

        // Register connection for broadcasts
        {
            let mut ss = shared_state.lock().unwrap();
            ss.tcp_connections.insert(addr, to_tcp_tx);
        }

        // TCP reader: TCP -> to_serial
        let stop_conn = stop_flag.clone();
        let reader_addr = addr;
        let tcp_reader = thread::spawn(move || -> Result<()> {
            let mut buffer = [0u8; 4096];
            while !stop_conn.load(Ordering::Relaxed) {
                match stream_reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        let buf = Bytes::copy_from_slice(&buffer[..n]);
                        if let Err(e) = to_serial_tx_conn.send(buf) {
                            warn!(?e, "Dropping data to serial, backpressure or shutdown");
                            break;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => {
                        warn!(?e, addr = %reader_addr, "TCP read error");
                        break;
                    }
                }
            }
            Ok(())
        });

        // TCP writer: from broadcast -> TCP
        let stop_conn = stop_flag.clone();
        let writer_addr = addr;
        let tcp_writer = thread::spawn(move || -> Result<()> {
            while !stop_conn.load(Ordering::Relaxed) {
                match to_tcp_rx.recv() {
                    Ok(buf) => {
                        if let Err(e) = stream_writer.write_all(&buf) {
                            warn!(?e, addr = %writer_addr, "TCP write error");
                            break;
                        }
                    }
                    Err(_e) => break,
                }
            }
            Ok(())
        });

        // Detach a supervisor for the connection
        let shared_state_remove = Arc::clone(&shared_state);
        thread::spawn(move || {
            let _ = tcp_reader.join();
            let _ = tcp_writer.join();
            if let Ok(mut ss) = shared_state_remove.lock() {
                ss.remove(&addr);
            }
            info!(%addr, "Closed connection");
        });
    }

    // Shutdown
    info!("Shutting down");
    if let Err(e) = serial_reader.join().unwrap_or(Ok(())) {
        warn!(?e, "Serial reader error on shutdown");
    }
    if let Err(e) = serial_writer.join().unwrap_or(Ok(())) {
        warn!(?e, "Serial writer error on shutdown");
    }
    if let Ok(mut ss) = shared_state.lock() {
        ss.dispose();
    }

    Ok(())
}
