#[cfg(not(target_arch = "wasm32"))]
use crate::protocol::ProtocolClient;

pub enum NetworkThreadMessage {
    #[cfg(not(target_arch = "wasm32"))]
    RunProtocolClient(Box<dyn ProtocolClient + Send + Sync>),

    Exit,
}

#[cfg(not(target_arch = "wasm32"))]
pub struct NetworkThread {
    pub control_tx: tokio::sync::mpsc::UnboundedSender<NetworkThreadMessage>,
}

#[cfg(not(target_arch = "wasm32"))]
impl NetworkThread {
    pub fn new(control_tx: tokio::sync::mpsc::UnboundedSender<NetworkThreadMessage>) -> Self {
        Self { control_tx }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_network_thread(
    mut control_rx: tokio::sync::mpsc::UnboundedReceiver<NetworkThreadMessage>,
) {
    loop {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                loop {
                    match control_rx.recv().await {
                        Some(NetworkThreadMessage::RunProtocolClient(mut client)) => {
                            tokio::spawn(async move {
                                client.run_connection().await.ok();
                            });
                        }
                        Some(NetworkThreadMessage::Exit) => return,
                        None => return,
                    }
                }
            })
    }
}

#[cfg(target_arch = "wasm32")]
pub struct NetworkThread {}

#[cfg(target_arch = "wasm32")]
pub fn run_network_thread() {}
