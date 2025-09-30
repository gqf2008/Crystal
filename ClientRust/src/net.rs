use std::net::SocketAddr;

use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::settings::NetworkSettings;

#[derive(Debug)]
pub enum NetworkEvent {
    Connected,
    Disconnected,
    Packet(Vec<u8>),
    Error(anyhow::Error),
}

pub struct NetworkStack {
    #[allow(dead_code)]
    sender: mpsc::Sender<Vec<u8>>,
    #[allow(dead_code)]
    receiver_task: JoinHandle<()>,
    events: mpsc::Receiver<NetworkEvent>,
}

impl NetworkStack {
    pub async fn connect(settings: &NetworkSettings) -> Result<Self> {
        let addr = format!("{}:{}", settings.ip_address, settings.port);
        let addr: SocketAddr = addr
            .parse()
            .with_context(|| format!("failed to parse server address `{}`", addr))?;

        let stream = TcpStream::connect(addr)
            .await
            .context("failed to connect to server")?;

        let (mut read_half, mut write_half) = stream.into_split();

        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(256);
        let (event_tx, event_rx) = mpsc::channel::<NetworkEvent>(256);

        let connected_tx = event_tx.clone();
        connected_tx.try_send(NetworkEvent::Connected).ok();

        let read_tx = event_tx.clone();
        let recv_task = tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                match read_half.read(&mut buf).await {
                    Ok(0) => {
                        let _ = read_tx.send(NetworkEvent::Disconnected).await;
                        break;
                    }
                    Ok(n) => {
                        if read_tx
                            .send(NetworkEvent::Packet(buf[..n].to_vec()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(err) => {
                        let _ = read_tx.send(NetworkEvent::Error(err.into())).await;
                        break;
                    }
                }
            }
        });

        let write_tx = event_tx.clone();
        let send_task = tokio::spawn(async move {
            while let Some(packet) = rx.recv().await {
                if let Err(err) = write_half.write_all(&packet).await {
                    let _ = write_tx.send(NetworkEvent::Error(err.into())).await;
                    break;
                }
            }
        });

        Ok(Self {
            sender: tx,
            receiver_task: tokio::spawn(async move {
                let _ = recv_task.await;
                let _ = send_task.await;
            }),
            events: event_rx,
        })
    }

    pub async fn next_event(&mut self) -> Option<NetworkEvent> {
        self.events.recv().await
    }
}
