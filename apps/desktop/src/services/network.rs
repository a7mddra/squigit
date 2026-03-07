use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Clone, Debug, PartialEq)]
pub enum NetworkStatus {
    Online,
    Offline,
    Poor,
}

#[derive(Clone, Debug)]
pub struct NetworkState {
    pub status: NetworkStatus,
    pub latency_ms: u64,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            status: NetworkStatus::Online,
            latency_ms: 0,
        }
    }
}

pub struct PeerNetworkMonitor {
    state: Arc<Mutex<NetworkState>>,
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

        tauri::async_runtime::spawn(async move {
            loop {
                let start = Instant::now();
                // 8.8.8.8:53 is Google DNS, very reliable.
                // Connect timeout of 2s.
                // This is a "TCP Ping".
                let status = match tokio::net::TcpStream::connect("8.8.8.8:53").await {
                    Ok(_) => {
                        let latency = start.elapsed().as_millis() as u64;
                        let status = if latency > 300 {
                            NetworkStatus::Poor
                        } else {
                            NetworkStatus::Online
                        };
                        NetworkState {
                            status,
                            latency_ms: latency,
                        }
                    }
                    Err(_) => NetworkState {
                        status: NetworkStatus::Offline,
                        latency_ms: 9999,
                    },
                };

                *state.lock().unwrap() = status;

                // Sleep for 2 seconds
                sleep(Duration::from_secs(2)).await;
            }
        });
    }
}
