// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum NetworkStatus {
    Online,
    Offline,
}

#[derive(Clone, Debug)]
pub(crate) struct NetworkState {
    pub status: NetworkStatus,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            status: NetworkStatus::Online,
        }
    }
}

pub(crate) struct PeerNetworkMonitor {
    state: Arc<Mutex<NetworkState>>,
}

impl Default for PeerNetworkMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerNetworkMonitor {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(NetworkState::default())),
        }
    }

    pub fn get_state(&self) -> NetworkState {
        self.state.lock().unwrap().clone()
    }

    pub fn start_monitor(&self) {
        let state = self.state.clone();

        std::thread::spawn(move || loop {
            let addr = std::net::SocketAddr::from(([8, 8, 8, 8], 53));
            let status = match std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
                Ok(_) => NetworkState {
                    status: NetworkStatus::Online,
                },
                Err(_) => NetworkState {
                    status: NetworkStatus::Offline,
                },
            };

            *state.lock().unwrap() = status;
            std::thread::sleep(Duration::from_secs(2));
        });
    }
}
