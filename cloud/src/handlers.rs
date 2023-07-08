use crate::{db, protocols, AppState};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{IntoResponse, Response},
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

        let parsed = protocols::ConnMsg::from_msg(&data);
        match parsed {
            Ok(msg) => {
                uid = msg.uid;
            }
            Err(_) => {
                return;
            }
        }
    } else {
        error!("Error receiving CONN message");
        return;
    }

    // Create a new connection in the database if it doesn't exist
    if db::get_connection(&state.pool, &uid).await.is_err() {
        if db::add_connection(&state.pool, &uid).await.is_err() {
            error!("Error adding new connection to database");
            return;
        }
    }

    // split socket into sender and receiver
    let (sender, receiver) = socket.split();

    tokio::spawn(ws_writer(sender, state.clone(), uid));
    tokio::spawn(ws_reader(receiver, state));
}

async fn ws_reader(mut receiver: SplitStream<WebSocket>, state: Arc<AppState>) {
    while let Some(Ok(msg)) = receiver.next().await {
        let data = msg.into_text().unwrap();
        info!("Received message: {:?}", data);

        let p = protocols::get_protocol(&data).unwrap_or(protocols::Protocol::INVALID);
        match p {
            // add sensor data to database
            protocols::Protocol::SENSOR => {
                let sensor_data_result = protocols::SensorMsg::from_msg(&data);

                match sensor_data_result {
                    Ok(sensor_data) => {
                        //process message in a separate thread, so that the connection is not blocked
                        let new_state = state.clone();
                        tokio::spawn(async move {
                            //add message to database
                            if db::add_received_message(&new_state.pool, &sensor_data)
                                .await
                                .is_err()
                            {
                                error!("Error adding sensor data to the db");
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
            protocols::Protocol::DISCONN => {
                error!("DISCONN is not yet properly implemented. Connection remains open");

                // let new_state = state.clone();
                // let new_uid = uid.clone();
                // tokio::spawn(async move {
                //     if db::delete_connection(&new_state.pool, &new_uid)
                //         .await
                //         .is_err()
                //         {
                //             error!("Error removing connection from database");
                //         }
                //     }
                // );
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

        //retrieve all undelivered messages from the queue
        let res = db::get_new_queued_messages(&state.pool).await;
        if res.is_err() {
            error!("Error getting connection from the db");
            continue;
        }
        let messages = res.unwrap();

        for msg in messages {
            // send AVG message to the client
            if sender
                .send(Message::Text(msg.message.to_string()))
                .await
                .is_err()
            {
                error!("Error sending message: {:?}", msg.message.to_string());
                return;
            }
            // add message to delivered messages
            if db::add_delivered_message(&state.pool, &uid, &msg.id)
                .await
                .is_err()
            {
                error!("Error adding delivered message to the db");
            }
            info!("Sent message: {:?}", msg.message.to_string());
        }
    }
}

pub async fn health_handler(State(state): State<Arc<AppState>>) -> Response {
    // retrieve metrics from the database
    let res = db::get_metrics(&state.pool).await;
    match res {
        Ok(metrics) => {
            let res_text = format!(
                r#"Status: OK
Number of connections: {} 
Number of received messages: {}
Number of unique queued messages: {}
Number of delivered messages: {}
                "#,
                metrics.connections.unwrap_or(-1),
                metrics.received_messages.unwrap_or(-1),
                metrics.queued_messages.unwrap_or(-1),
                metrics.delivered_messages.unwrap_or(-1),
            );
            info!("Health check: ok");
            return res_text.into_response();
        }
        Err(_) => {
            error!("Error getting database metrics");
            return "Status: Error".into_response();
        }
    }
}
