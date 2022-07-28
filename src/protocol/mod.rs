use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolClientError {
    #[error("client initiated disconnect")]
    ClientInitiatedDisconnect,
}

#[async_trait]
pub trait ProtocolClient {
    async fn run_connection(&mut self) -> Result<(), anyhow::Error>;
}

macro_rules! implement_protocol_client {
    ( $x:ident ) => {
        #[async_trait]
        impl ProtocolClient for $x {
            async fn run_connection(&mut self) -> Result<(), anyhow::Error> {
                let socket = TcpStream::connect(&self.server_address).await?;
                let mut connection = Connection::new(socket, self.packet_codec.as_ref());

                loop {
                    tokio::select! {
                        packet = connection.read_packet() => {
                            match packet {
                                Ok(packet) => {
                                    match self.handle_packet(&packet).await {
                                        Ok(_) => {},
                                        Err(error) => {
                                            log::warn!("RECV [{:03X}] {:02x?}", packet.command, &packet.data[..]);
                                            return Err(error);
                                        },
                                    }
                                },
                                Err(error) => {
                                    return Err(error);
                                }
                            }
                        },
                        server_message = self.client_message_rx.recv() => {
                            if let Some(message) = server_message {
                                self.handle_client_message(&mut connection, message).await?;
                            } else {
                                return Err(ProtocolClientError::ClientInitiatedDisconnect.into());
                            }
                        }
                    };
                }

                // Ok(())
            }
        }
    };
}

pub mod irose;
pub mod narose667;

pub enum ProtocolType {
    Irose,
    Narose667,
}

impl Default for ProtocolType {
    fn default() -> Self {
        Self::Irose
    }
}
