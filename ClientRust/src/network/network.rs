use std::net::SocketAddr;

use anyhow::{anyhow, Context, Result};
use byteorder::{ByteOrder, LittleEndian};
use mir2_shared::packets::{serialize_packet, PacketHeader, PacketMessage};
use mir2_shared::SharedError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::settings::NetworkSettings;

#[derive(Debug)]
pub enum NetworkEvent {
    Connected,
    Disconnected,
    Packet {
        header: PacketHeader,
        payload: Vec<u8>,
    },
    Error(anyhow::Error),
}

pub struct NetworkStack {
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
            let mut buffer = Vec::with_capacity(8192);
            loop {
                match read_half.read(&mut buf).await {
                    Ok(0) => {
                        let _ = read_tx.send(NetworkEvent::Disconnected).await;
                        break;
                    }
                    Ok(n) => {
                        buffer.extend_from_slice(&buf[..n]);

                        loop {
                            if buffer.len() < PacketHeader::HEADER_SIZE {
                                break;
                            }

                            let length = LittleEndian::read_u16(&buffer[0..2]) as usize;
                            if length < PacketHeader::HEADER_SIZE {
                                let err = SharedError::InvalidPacketLength(length as u16);
                                let _ = read_tx
                                    .send(NetworkEvent::Error(anyhow::Error::new(err)))
                                    .await;
                                return;
                            }

                            if buffer.len() < length {
                                break;
                            }

                            let opcode = LittleEndian::read_i16(&buffer[2..4]);
                            let payload = buffer[PacketHeader::HEADER_SIZE..length].to_vec();
                            buffer.drain(..length);

                            let header = PacketHeader::new(length as u16, opcode);
                            if read_tx
                                .send(NetworkEvent::Packet { header, payload })
                                .await
                                .is_err()
                            {
                                return;
                            }
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

    pub async fn send_raw(&self, data: Vec<u8>) -> Result<()> {
        self.sender
            .send(data)
            .await
            .map_err(|err| anyhow!("network send failed: {err}"))
    }

    pub async fn send_packet<P: PacketMessage>(&self, packet: &P) -> Result<()> {
        let mut buffer = Vec::new();
        serialize_packet(&mut buffer, packet).map_err(anyhow::Error::new)?;
        self.send_raw(buffer).await
    }
}
