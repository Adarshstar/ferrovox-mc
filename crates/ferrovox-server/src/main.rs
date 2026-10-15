use ferrovox_core::{versions, ServerConfig};
use std::sync::Arc;
use tokio::net::{TcpListener, UdpSocket};
use tracing::{error, info};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = Arc::new(ServerConfig::load_or_create("ferrovox.toml")?);

    info!(
        java_version = versions::JAVA_EDITION_NAME,
        java_protocol = versions::JAVA_PROTOCOL_VERSION,
        bedrock_version = versions::BEDROCK_EDITION_NAME,
        bedrock_protocol = versions::BEDROCK_PROTOCOL_VERSION,
        "FerroVox starting"
    );

    let rt = ferrovox_core::build_runtime(&cfg)?;
    rt.block_on(run(cfg))
}

async fn run(cfg: Arc<ServerConfig>) -> anyhow::Result<()> {
    let mut tasks = Vec::new();

    if cfg.java.enabled {
        let cfg = cfg.clone();
        let addr = format!("{}:{}", cfg.java.bind, cfg.java.port);
        tasks.push(tokio::spawn(async move {
            if let Err(e) = run_java(addr, cfg).await {
                error!(error = %e, "Java Edition listener stopped");
            }
        }));
    }

    if cfg.bedrock.enabled {
        let cfg = cfg.clone();
        let addr = format!("{}:{}", cfg.bedrock.bind, cfg.bedrock.port);
        tasks.push(tokio::spawn(async move {
            if let Err(e) = run_bedrock(addr, cfg).await {
                error!(error = %e, "Bedrock listener stopped");
            }
        }));
    }

    for t in tasks {
        let _ = t.await;
    }
    Ok(())
}

async fn run_java(addr: String, cfg: Arc<ServerConfig>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    info!(%addr, "Java Edition listener bound");

    loop {
        let (socket, peer) = listener.accept().await?;
        let cfg = cfg.clone();
        tokio::spawn(async move {
            tracing::debug!(%peer, "Java Edition connection accepted");
            if let Err(e) = ferrovox_protocol_java::handle_connection(socket, cfg).await {
                tracing::debug!(%peer, error = %e, "Java Edition connection ended");
            }
        });
    }
}

async fn run_bedrock(addr: String, cfg: Arc<ServerConfig>) -> anyhow::Result<()> {
    use ferrovox_protocol_bedrock::{build_unconnected_pong, parse_unconnected_ping};

    let socket = Arc::new(UdpSocket::bind(&addr).await?);
    info!(%addr, "Bedrock Edition listener bound (RakNet offline handshake only, Phase 0)");

    let server_guid: i64 = rand::random();
    let mut buf = [0u8; 2048];

    loop {
        let (len, peer) = socket.recv_from(&mut buf).await?;
        if let Some(ping) = parse_unconnected_ping(&buf[..len]) {
            let pong = build_unconnected_pong(
                &ping,
                server_guid,
                &cfg.server.motd,
                cfg.bedrock.port,
                cfg.server.max_players,
            );
            let _ = socket.send_to(&pong, peer).await;
        }
    }
}
