//! # Native WebSocket Live Sync Client
//!
//! Dedicated background thread client using `tungstenite` with auto-reconnect and packet routing.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use tungstenite::protocol::Message;
use url::Url;

use crate::protocol::{decode_packet, Packet};

/// Commands sent from the session controller to the network client thread.
#[derive(Debug)]
pub enum ClientCommand {
    /// Send raw binary packet to server.
    Send(Vec<u8>),
    /// Terminate connection and worker thread.
    Stop,
}

/// Network client status messages.
#[derive(Debug, Clone)]
pub enum ClientMessage {
    Status(String),
    PacketReceived(Packet),
    Disconnected,
}

/// Manages the background WebSocket transport thread.
pub struct SyncClient {
    running: Arc<AtomicBool>,
    cmd_sender: Sender<ClientCommand>,
    thread_handle: Option<JoinHandle<()>>,
}

impl SyncClient {
    /// Spawns a new background WebSocket client connecting to `url`.
    pub fn connect(
        url_str: String,
        msg_sender: Sender<ClientMessage>,
        auto_reconnect: bool,
        max_reconnect_attempts: usize,
    ) -> Result<Self, String> {
        let _parsed = Url::parse(&url_str).map_err(|e| format!("Invalid WebSocket URL '{}': {}", url_str, e))?;

        let running = Arc::new(AtomicBool::new(true));
        let (cmd_sender, cmd_receiver) = crossbeam_channel::unbounded::<ClientCommand>();

        let running_clone = running.clone();
        let handle = thread::spawn(move || {
            Self::run_loop(
                url_str,
                msg_sender,
                cmd_receiver,
                running_clone,
                auto_reconnect,
                max_reconnect_attempts,
            );
        });

        Ok(Self {
            running,
            cmd_sender,
            thread_handle: Some(handle),
        })
    }

    /// Sends a binary packet to the connected server.
    pub fn send_packet(&self, data: Vec<u8>) -> Result<(), String> {
        self.cmd_sender
            .send(ClientCommand::Send(data))
            .map_err(|e| e.to_string())
    }

    /// Stops the client thread cleanly.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        let _ = self.cmd_sender.send(ClientCommand::Stop);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    fn run_loop(
        url_str: String,
        msg_sender: Sender<ClientMessage>,
        cmd_receiver: Receiver<ClientCommand>,
        running: Arc<AtomicBool>,
        auto_reconnect: bool,
        max_reconnect_attempts: usize,
    ) {
        let mut attempts = 0;

        while running.load(Ordering::Relaxed) {
            if attempts > 0 {
                let _ = msg_sender.send(ClientMessage::Status(format!(
                    "RECONNECTING... ({}/{})",
                    attempts, max_reconnect_attempts
                )));
            } else {
                let _ = msg_sender.send(ClientMessage::Status("CONNECTING...".to_string()));
            }

            match tungstenite::connect(&url_str) {
                Ok((mut socket, _response)) => {
                    attempts = 0;
                    let _ = msg_sender.send(ClientMessage::Status("CONNECTED".to_string()));

                    // Set underlying TCP stream read timeout for non-blocking command checking
                    if let tungstenite::stream::MaybeTlsStream::Plain(ref s) = socket.get_ref() {
                        let _ = s.set_read_timeout(Some(Duration::from_millis(100)));
                    }

                    while running.load(Ordering::Relaxed) {
                        // Check for outgoing commands
                        while let Ok(cmd) = cmd_receiver.try_recv() {
                            match cmd {
                                ClientCommand::Send(data) => {
                                    if let Err(e) = socket.send(Message::Binary(data.into())) {
                                        let _ = msg_sender.send(ClientMessage::Status(format!(
                                            "Send error: {}",
                                            e
                                        )));
                                        break;
                                    }
                                }
                                ClientCommand::Stop => {
                                    let _ = socket.close(None);
                                    let _ = msg_sender.send(ClientMessage::Disconnected);
                                    return;
                                }
                            }
                        }

                        // Read incoming messages
                        match socket.read() {
                            Ok(Message::Binary(bin)) => match decode_packet(&bin) {
                                Ok(packet) => {
                                    let _ = msg_sender.send(ClientMessage::PacketReceived(packet));
                                }
                                Err(e) => {
                                    let _ = msg_sender.send(ClientMessage::Status(format!(
                                        "Packet decode error: {}",
                                        e
                                    )));
                                }
                            },
                            Ok(Message::Ping(payload)) => {
                                let _ = socket.send(Message::Pong(payload));
                            }
                            Ok(Message::Close(_)) => {
                                let _ = msg_sender.send(ClientMessage::Status(
                                    "WebSocket closed by server".to_string(),
                                ));
                                break;
                            }
                            Ok(_) => {}
                            Err(tungstenite::Error::Io(ref e))
                                if e.kind() == std::io::ErrorKind::WouldBlock
                                    || e.kind() == std::io::ErrorKind::TimedOut =>
                            {
                                // Normal timeout: continue poll loop
                                continue;
                            }
                            Err(e) => {
                                let _ = msg_sender.send(ClientMessage::Status(format!(
                                    "Connection error: {}",
                                    e
                                )));
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    attempts += 1;
                    if !auto_reconnect || attempts > max_reconnect_attempts {
                        let _ = msg_sender.send(ClientMessage::Status(format!(
                            "DISCONNECTED (Connection failed: {})",
                            e
                        )));
                        break;
                    }
                    let _ = msg_sender.send(ClientMessage::Status(format!(
                        "Connection failed: {}. Retrying ({}/{})...",
                        e, attempts, max_reconnect_attempts
                    )));
                }
            }

            if running.load(Ordering::Relaxed) && auto_reconnect && attempts <= max_reconnect_attempts {
                thread::sleep(Duration::from_millis(1500));
            } else {
                break;
            }
        }

        let _ = msg_sender.send(ClientMessage::Disconnected);
    }
}

impl Drop for SyncClient {
    fn drop(&mut self) {
        self.stop();
    }
}
