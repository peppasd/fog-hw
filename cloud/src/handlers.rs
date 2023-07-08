use crate::{db, protocol, AppState};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};
use std::sync::Arc;
use tracing::{error, info};

pub async fn handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    info!("New websocket connection");
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let uid: String;

    //get initial message with id
    if let Some(Ok(msg)) = socket.next().await {
        let data = msg.into_text().unwrap();
        info!("Received message: {:?}", data);

        let parsed = protocol::ConnMsg::from_msg(&data);
        match parsed {
            Ok(msg) => {
                uid = msg.uid;
                info!("New connection from id: {:?}", uid);
            }
            Err(_) => {
                return;
            }
        }
    } else {
        error!("Error receiving initial message");
        return;
    }

    // check if id is already in database
    if db::get_connection(&state.pool, &uid).await.is_err() {
        // add new id to database
        if db::add_connection(&state.pool, &uid).await.is_err() {
            error!("Error adding new connection to database");
            return;
        }
    }

    // split socket into sender and receiver
    let (sender, receiver) = socket.split();

    tokio::spawn(ws_writer(sender, state.clone(), uid.clone()));
    tokio::spawn(ws_reader(receiver, state, uid));
}

async fn ws_reader(mut receiver: SplitStream<WebSocket>, state: Arc<AppState>, uid: String) {
    while let Some(Ok(msg)) = receiver.next().await {
        let data = msg.into_text().unwrap();
        info!("Received message: {:?}", data);

        let p = protocol::get_protocol(&data).unwrap_or(protocol::Protocol::INVALID);
        match p {
            // add sensor data to database
            protocol::Protocol::SENSOR => {
                let sensor_data_result = protocol::SensorMsg::from_msg(&data);

                match sensor_data_result {
                    Ok(sensor_data) => {
                        let new_state = state.clone();
                        tokio::spawn(async move {
                            //add message to database
                            if db::add_received_message(&new_state.pool, &sensor_data)
                                .await
                                .is_err()
                            {
                                error!("Error adding sensor data to database");
                            }
                            //update last seen timestamp
                            if db::update_connection(&new_state.pool, &sensor_data.uid)
                                .await
                                .is_err()
                            {
                                error!("Error updating last seen timestamp");
                            }
                        });
                    }
                    Err(_) => {
                        error!("Invalid protocol: {:?}", data.to_string());
                        return;
                    }
                }
            }
            protocol::Protocol::DISCONN => {
                let new_state = state.clone();
                let new_uid = uid.clone();
                tokio::spawn(async move {
                    if db::delete_connection(&new_state.pool, &new_uid)
                        .await
                        .is_err()
                    {
                        error!("Error removing connection from database");
                    }
                });
            }
            _ => {
                error!("Invalid protocol: {:?}", data.to_string());
                return;
            }
        }
    }
}

async fn ws_writer(mut sender: SplitSink<WebSocket, Message>, state: Arc<AppState>, uid: String) {
    // sending rate is 1 message per x seconds
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

    loop {
        interval.tick().await;

        let res = db::get_connection(&state.pool, &uid).await;
        if res.is_err() {
            error!("Error getting connection from database");
            continue;
        }
        let last_seen = res.unwrap().last_seen;

        let res2 = db::get_new_queued_messages(&state.pool, &last_seen).await;
        if res2.is_err() {
            error!("Error getting connection from database");
            continue;
        }
        let messages = res2.unwrap();

        for msg in messages {
            if sender
                .send(Message::Text(msg.message.to_string()))
                .await
                .is_err()
            {
                error!("Error sending message: {:?}", msg.message.to_string());
                return;
            }
            info!("Sent message: {:?}", msg.message.to_string());
        }

        if db::update_connection(&state.pool, &uid).await.is_err() {
            error!("Error updating last seen timestamp");
        }
    }
}

pub async fn health_handler(State(state): State<Arc<AppState>>) -> &'static str {
    db::add_connection(&state.pool, "123").await.expect("hiii");
    info!("Health check");
    "ok"
}
