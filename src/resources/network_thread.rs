use crate::protocol::ProtocolClient;

pub enum NetworkThreadMessage {
    RunProtocolClient(Box<dyn ProtocolClient + Send + Sync>),
    Exit,
}

pub struct NetworkThread {
    pub control_tx: tokio::sync::mpsc::UnboundedSender<NetworkThreadMessage>,
}

impl NetworkThread {
    pub fn new(control_tx: tokio::sync::mpsc::UnboundedSender<NetworkThreadMessage>) -> Self {
        Self { control_tx }
    }
}

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
