use std::{io::Error, net::Ipv4Addr, sync::Arc};

use tokio::{
    net::TcpListener,
    sync::{
        broadcast::{self, Sender},
        Mutex,
    },
    time,
};

use crate::{game::game_state::GameState, utils::logger::Logger};

use super::{
    client::{Client, CLIENTS},
    protocol::{MessageType, Packet},
};

static HOST: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

pub struct ServerInstance {
    pub socket: TcpListener,
    pub game_state: GameState,
}

impl ServerInstance {
    /// Creates and binds a new `ServerInstance` to the given port.
    ///
    /// On success, returns an initialized server with a bound TCP listener.
    /// Returns an error if the bind fails.
    pub async fn create_instance(port: u16) -> Result<ServerInstance, Error> {
        return match TcpListener::bind((HOST, port)).await {
            Ok(tcp_stream) => {
                Logger::debug(&format!("Server listening on port {port}"));
                Ok(ServerInstance {
                    socket: tcp_stream,
                    game_state: GameState::new_game(),
                })
            }
            Err(error) => Err(error),
        };
    }

    /// Starts the main server loop and handles incoming client connections.
    ///
    /// - Spawns a background task to broadcast game state updates.
    /// - Accepts new TCP clients, logs them, registers them, and spawns their handling task.
    ///
    /// Runs indefinitely. Requires `self` as `Arc` for shared access.
    pub async fn run(self: Arc<Self>) {
        let (tx, _) = broadcast::channel::<Packet>(10);
        let transmiter = Arc::new(Mutex::new(tx));

        tokio::spawn({
            let server_clone = Arc::clone(&self);
            let tx = Arc::clone(&transmiter);
            async move { ServerInstance::write_state_update(tx, server_clone).await }
        });

        loop {
            let tx = Arc::clone(&transmiter);
            if let Ok((c_stream, addr)) = self.socket.accept().await {
                Logger::info(&format!("{addr}: received request"));
                let tx = tx.lock().await.subscribe();
                let mut clients = CLIENTS.write().await;
                let client = Client::new(c_stream, addr, tx);
                clients.insert(addr, Arc::clone(&client));
                tokio::spawn(async move {
                    client.connect().await;
                });
            }
        }
    }

    /// Broadcasts the current game state to all connected clients every second.
    ///
    /// On each tick:
    /// - If clients are connected, wraps the game state in a `Packet`
    ///   and sends it through the broadcast channel.
    /// - Skips sending if no clients are present.
    ///
    /// # Arguments
    ///
    /// * `tx` - Broadcast sender wrapped in a mutex.
    /// * `server` - Shared server reference for accessing game state.
    ///
    /// Intended to run as a background task. Never returns under normal conditions.
    async fn write_state_update(tx: Arc<Mutex<Sender<Packet>>>, server: Arc<ServerInstance>) {
        let mut interval = time::interval(std::time::Duration::from_millis(1000));
        loop {
            interval.tick().await;
            let clients = CLIENTS.read().await;

            if clients.len() > 0 {
                Logger::info(&format!("Sending game state"));
                let payload = server.game_state.wrap_game_state();
                let packet = Packet::new(MessageType::GAMESTATE, &payload);
                let tx = tx.lock().await;
                let _ = tx.send(packet);
            }
        }
    }
}
