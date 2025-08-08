use std::collections::HashMap;
use std::net::SocketAddr;

use bytes::Bytes;
use crossbeam_channel as channel;

pub struct SharedState {
    pub tcp_connections: HashMap<SocketAddr, channel::Sender<Bytes>>, // outbound to TCP
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            tcp_connections: HashMap::new(),
        }
    }

    pub fn remove(&mut self, addr: &SocketAddr) {
        self.tcp_connections.remove(addr);
    }

    pub fn dispose(&mut self) {
        self.tcp_connections.clear();
    }

    pub fn broadcast(&mut self, data: Bytes) {
        let mut to_remove: Vec<SocketAddr> = Vec::new();
        for (addr, tx) in self.tcp_connections.iter() {
            if let Err(_err) = tx.send(data.clone()) {
                to_remove.push(*addr);
            }
        }
        for addr in to_remove {
            self.remove(&addr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broadcast_removes_dead_receivers() {
        let (tx_alive, rx_alive) = channel::bounded::<Bytes>(1);
        let (tx_dead, _rx_dead) = channel::bounded::<Bytes>(1);
        drop(_rx_dead); // drop to simulate dead receiver

        let mut state = SharedState::new();
        let a1: SocketAddr = "127.0.0.1:10000".parse().unwrap();
        let a2: SocketAddr = "127.0.0.1:10001".parse().unwrap();
        state.tcp_connections.insert(a1, tx_alive);
        state.tcp_connections.insert(a2, tx_dead);

        state.broadcast(Bytes::from_static(b"hello"));

        // Alive should receive
        assert_eq!(rx_alive.recv().unwrap(), Bytes::from_static(b"hello"));
        // Dead should be removed
        assert!(!state.tcp_connections.contains_key(&a2));
    }
}
