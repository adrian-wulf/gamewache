use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::{broadcast, RwLock};
use tracing::warn;

use crate::models::{LiveTweakMessage, TweakProperty};

#[derive(Clone)]
pub struct LiveTweakerHub {
    properties: Arc<RwLock<HashMap<String, TweakProperty>>>,
    broadcast_tx: broadcast::Sender<String>,
}

impl LiveTweakerHub {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(256);
        Self {
            properties: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
        }
    }

    pub async fn handle_socket(&self, socket: WebSocket) {
        let (mut sender, mut receiver) = socket.split();
        let mut broadcast_rx = self.broadcast_tx.subscribe();

        // Wyślij bieżący stan wszystkich zarejestrowanych zmiennych do nowego klienta
        {
            let props = self.properties.read().await;
            for prop in props.values() {
                let init_msg = json!({
                    "msg_type": "register",
                    "param_path": prop.path,
                    "value": prop.value,
                    "meta": {
                        "min": prop.min,
                        "max": prop.max,
                        "step": prop.step,
                        "prop_type": prop.prop_type
                    }
                });
                if sender.send(Message::Text(init_msg.to_string().into())).await.is_err() {
                    return;
                }
            }
        }

        let hub = self.clone();

        // Task wysyłający wiadomości z broadcastu do klienta
        let mut send_task = tokio::spawn(async move {
            while let Ok(msg) = broadcast_rx.recv().await {
                if sender.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        });

        // Task odbierający wiadomości od klienta
        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(message)) = receiver.next().await {
                if let Message::Text(text) = message {
                    hub.process_incoming(&text).await;
                }
            }
        });

        // Jeśli którykolwiek task zakończy pracę, kończymy połączenie
        tokio::select! {
            _ = (&mut send_task) => recv_task.abort(),
            _ = (&mut recv_task) => send_task.abort(),
        };
    }

    async fn process_incoming(&self, text: &str) {
        let parsed: Result<LiveTweakMessage, _> = serde_json::from_str(text);
        let msg = match parsed {
            Ok(m) => m,
            Err(e) => {
                warn!("Invalid WebSocket live tweak message: {}", e);
                return;
            }
        };

        match msg.msg_type.as_str() {
            "register" => {
                if let (Some(path), Some(value)) = (msg.param_path, msg.value) {
                    let (min, max, step, prop_type) = if let Some(meta) = msg.meta {
                        (
                            meta.get("min").and_then(|v| v.as_f64()),
                            meta.get("max").and_then(|v| v.as_f64()),
                            meta.get("step").and_then(|v| v.as_f64()),
                            meta.get("prop_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("float")
                                .to_string(),
                        )
                    } else {
                        (None, None, None, "float".to_string())
                    };

                    let prop = TweakProperty {
                        path: path.clone(),
                        value: value.clone(),
                        min,
                        max,
                        step,
                        prop_type,
                    };

                    self.properties.write().await.insert(path, prop);
                    // Przekaż rejestrację do WebUI i pozostałych klientów
                    let _ = self.broadcast_tx.send(text.to_string());
                }
            }
            "tweak" => {
                if let (Some(path), Some(value)) = (&msg.param_path, &msg.value) {
                    let mut props = self.properties.write().await;
                    if let Some(prop) = props.get_mut(path) {
                        prop.value = value.clone();
                    }
                    // Rozgłoś zmianę do gry i UI
                    let _ = self.broadcast_tx.send(text.to_string());
                }
            }
            "command" => {
                // Rozgłoś wykonanie komendy konsolowej do gry
                let _ = self.broadcast_tx.send(text.to_string());
            }
            _ => {}
        }
    }

    pub async fn tweak_var(&self, path: &str, value: serde_json::Value) -> bool {
        let mut props = self.properties.write().await;
        if let Some(prop) = props.get_mut(path) {
            prop.value = value.clone();
            let msg = json!({
                "msg_type": "tweak",
                "param_path": path,
                "value": value
            });
            let _ = self.broadcast_tx.send(msg.to_string());
            true
        } else {
            // Możemy też zarejestrować i wysłać
            let prop = TweakProperty {
                path: path.to_string(),
                value: value.clone(),
                min: None,
                max: None,
                step: None,
                prop_type: "dynamic".to_string(),
            };
            props.insert(path.to_string(), prop);
            let msg = json!({
                "msg_type": "tweak",
                "param_path": path,
                "value": value
            });
            let _ = self.broadcast_tx.send(msg.to_string());
            true
        }
    }

    pub async fn send_command(&self, cmd: &str) -> usize {
        let msg = json!({
            "msg_type": "command",
            "cmd": cmd
        });
        self.broadcast_tx.send(msg.to_string()).unwrap_or(0)
    }

    pub async fn get_tweakables(&self) -> Vec<TweakProperty> {
        self.properties.read().await.values().cloned().collect()
    }
}
