use crate::config::ServerConfig;
use crate::plugin::PluginManager;
use crate::protocol::*;
use colored::*;
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

pub const SERVER_NAME: &str = "CrabMC";
pub const SERVER_VERSION: &str = "0.1.0-alpha";
pub const MINECRAFT_VERSION: &str = "1.20.4";
pub const PROTOCOL_VERSION: i32 = 765;

pub struct CrabServer {
    config: Arc<ServerConfig>,
    plugin_mgr: Arc<Mutex<PluginManager>>,
    online_players: Arc<AtomicU32>,
    running: Arc<AtomicBool>,
}

impl CrabServer {
    pub fn new(config: ServerConfig) -> Self {
        let mut plugin_mgr = PluginManager::new("./plugins");
        plugin_mgr.register_builtin_plugins();

        Self {
            config: Arc::new(config),
            plugin_mgr: Arc::new(Mutex::new(plugin_mgr)),
            online_players: Arc::new(AtomicU32::new(0)),
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn print_banner(&self) {
        println!("{}", r#"
  ___           _      __  __  ____ 
 / __|_ _ __ _ | |__  |  \/  |/ ___|  (Rust Edition)
| (__| '_/ _` || '_ \ | |\/| | |      PaperMC Architecture in Rust
 \___|_| \__,_||_.__/ |_|  |_|\____|
"#.bright_red().bold());

        println!(
            "{} {} (MC: {}) - Protocol {}",
            "Running:".bright_yellow(),
            format!("{} v{}", SERVER_NAME, SERVER_VERSION).bright_cyan().bold(),
            MINECRAFT_VERSION.bright_green(),
            PROTOCOL_VERSION
        );
        println!(
            "{} {}",
            "GitHub:".bright_yellow(),
            "https://github.com/myzert/crabmc-server".underline()
        );
        println!("--------------------------------------------------");
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        self.print_banner();

        // Enable plugins
        {
            let mut mgr = self.plugin_mgr.lock().await;
            mgr.enable_all(SERVER_VERSION);
        }

        let bind_addr = format!("{}:{}", self.config.server_ip, self.config.server_port);
        let listener = TcpListener::bind(&bind_addr).await?;
        info!("Starting Minecraft server on {}", bind_addr.bright_green().bold());
        info!("PaperMC-compatible MOTD: {}", self.config.motd);
        info!("Max players: {}", self.config.max_players);
        info!("Online mode: {}", self.config.online_mode);
        info!("Server is ready for connections! Type 'help' for commands, 'stop' to shutdown.");

        let running_clone = self.running.clone();
        let plugin_mgr_clone = self.plugin_mgr.clone();

        // Spawn Console Input Handler (Stdin)
        tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let mut reader = BufReader::new(stdin).lines();

            while running_clone.load(Ordering::SeqCst) {
                match reader.next_line().await {
                    Ok(Some(line)) => {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }

                        let parts: Vec<&str> = line.split_whitespace().collect();
                        let cmd = parts[0].to_lowercase();
                        let args = &parts[1..];

                        match cmd.as_str() {
                            "stop" => {
                                info!("Stopping the server...");
                                running_clone.store(false, Ordering::SeqCst);
                                break;
                            }
                            "help" => {
                                info!("--- CrabMC Help ---");
                                info!("stop    - Gracefully stops the server");
                                info!("plugins - Lists all loaded plugins");
                                info!("version - Displays CrabMC and Minecraft version");
                                info!("ping    - Tests plugin command dispatcher");
                                info!("crab    - CrabMC fun command");
                                info!("help    - Shows this help message");
                            }
                            "plugins" | "pl" => {
                                let mgr = plugin_mgr_clone.lock().await;
                                let list = mgr.list_plugins();
                                info!("Plugins ({}): {}", list.len(), list.join(", ").bright_green());
                            }
                            "version" | "ver" => {
                                info!(
                                    "This server is running {} version {} (Implementing Minecraft {})",
                                    SERVER_NAME.bright_cyan(),
                                    SERVER_VERSION.bright_yellow(),
                                    MINECRAFT_VERSION.bright_green()
                                );
                            }
                            _ => {
                                let mut mgr = plugin_mgr_clone.lock().await;
                                if !mgr.dispatch_command("CONSOLE", &cmd, args) {
                                    warn!("Unknown command. Type 'help' for help.");
                                }
                            }
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        error!("Error reading console input: {}", e);
                        break;
                    }
                }
            }
        });

        // Main network accept loop
        while self.running.load(Ordering::SeqCst) {
            tokio::select! {
                res = listener.accept() => {
                    match res {
                        Ok((stream, addr)) => {
                            let cfg = self.config.clone();
                            let online = self.online_players.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, cfg, online).await {
                                    // Debug info for client disconnects
                                    tracing::debug!("Client {} disconnected: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Error accepting connection: {}", e);
                        }
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(200)) => {
                    // Check if server was stopped via console
                    if !self.running.load(Ordering::SeqCst) {
                        break;
                    }
                }
            }
        }

        info!("Shutting down CrabMC...");
        {
            let mut mgr = self.plugin_mgr.lock().await;
            mgr.disable_all();
        }
        info!("Server shutdown complete. Goodbye!");
        Ok(())
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    config: Arc<ServerConfig>,
    online_count: Arc<AtomicU32>,
) -> anyhow::Result<()> {
    // Read VarInt length of handshake packet
    let packet_len = read_async_varint(&mut stream).await?;
    if packet_len <= 0 || packet_len > 32767 {
        return Ok(());
    }

    let mut packet_buf = vec![0u8; packet_len as usize];
    stream.read_exact(&mut packet_buf).await?;

    let mut cursor = Cursor::new(&packet_buf[..]);
    let packet_id = cursor.read_varint()?;
    if packet_id != 0x00 {
        return Ok(()); // Handshake packet ID must be 0x00
    }

    let handshake = HandshakePacket::decode(&mut cursor)?;

    match handshake.next_state {
        ConnectionState::Status => {
            handle_status_state(&mut stream, &config, &online_count).await?;
        }
        ConnectionState::Login => {
            handle_login_state(&mut stream, &handshake).await?;
        }
        _ => {}
    }

    Ok(())
}

async fn handle_status_state(
    stream: &mut TcpStream,
    config: &ServerConfig,
    online_count: &Arc<AtomicU32>,
) -> anyhow::Result<()> {
    loop {
        let packet_len = match read_async_varint(stream).await {
            Ok(len) => len,
            Err(_) => return Ok(()),
        };

        if packet_len <= 0 {
            return Ok(());
        }

        let mut packet_buf = vec![0u8; packet_len as usize];
        stream.read_exact(&mut packet_buf).await?;

        let mut cursor = Cursor::new(&packet_buf[..]);
        let packet_id = cursor.read_varint()?;

        match packet_id {
            0x00 => {
                // Status Request -> send Status Response
                let current_online = online_count.load(Ordering::Relaxed) as i32;
                let status_json = StatusResponseJson {
                    version: ServerVersionInfo {
                        name: format!("CrabMC {}", MINECRAFT_VERSION),
                        protocol: PROTOCOL_VERSION,
                    },
                    players: PlayersInfo {
                        max: config.max_players,
                        online: current_online,
                        sample: vec![],
                    },
                    description: DescriptionInfo {
                        text: config.motd.clone(),
                    },
                    enforces_secure_chat: false,
                    previews_chat: false,
                    favicon: None,
                };

                let json_str = serde_json::to_string(&status_json)?;
                let response_bytes = encode_status_response(&json_str);
                stream.write_all(&response_bytes).await?;
                stream.flush().await?;
            }
            0x01 => {
                // Ping Packet -> Send Pong Packet with the exact same payload
                let mut pong = Vec::new();
                pong.write_varint(packet_buf.len() as i32)?;
                pong.extend_from_slice(&packet_buf);
                stream.write_all(&pong).await?;
                stream.flush().await?;
                return Ok(());
            }
            _ => return Ok(()),
        }
    }
}

async fn handle_login_state(
    stream: &mut TcpStream,
    handshake: &HandshakePacket,
) -> anyhow::Result<()> {
    // Read Login Start packet (0x00)
    let packet_len = read_async_varint(stream).await?;
    let mut packet_buf = vec![0u8; packet_len as usize];
    stream.read_exact(&mut packet_buf).await?;

    let mut cursor = Cursor::new(&packet_buf[..]);
    let packet_id = cursor.read_varint()?;

    if packet_id == 0x00 {
        let player_name = cursor.read_minecraft_string().unwrap_or_else(|_| "Player".to_string());
        info!("Player '{}' attempting to connect (protocol: {})...", player_name.bright_cyan(), handshake.protocol_version);

        let kick_msg = serde_json::json!({
            "text": "\n🦀 §6§lCrabMC Server (PaperMC in Rust)\n\n§aConnected successfully to Rust engine!\n§7Server version: §b0.1.0-alpha §7(MC 1.20.4)\n\n§eWorld simulation and full play state are under active development.\n§dStar us on GitHub: §fgithub.com/myzert/crabmc-server\n"
        });

        let disconnect_bytes = encode_disconnect_login(&kick_msg.to_string());
        stream.write_all(&disconnect_bytes).await?;
        stream.flush().await?;
    }

    Ok(())
}

async fn read_async_varint(stream: &mut TcpStream) -> std::io::Result<i32> {
    let mut num_read = 0;
    let mut result = 0i32;

    loop {
        let mut buf = [0u8; 1];
        stream.read_exact(&mut buf).await?;
        let read = buf[0];
        let value = (read & 0b0111_1111) as i32;
        result |= value << (7 * num_read);

        num_read += 1;
        if num_read > 5 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "VarInt is too big",
            ));
        }

        if (read & 0b1000_0000) == 0 {
            break;
        }
    }

    Ok(result)
}
