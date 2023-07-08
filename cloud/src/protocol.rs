use std::{
    error::Error,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{error, log::warn};

use crate::db;

pub enum Protocol {
    CONN,
    SENSOR,
    AVG,
    DISCONN,
    INVALID,
}

pub fn get_protocol(msg: &String) -> Result<Protocol, Box<dyn Error>> {
    let parts: Vec<&str> = msg.split("#").collect();

    match parts[0] {
        "CONN" => Ok(Protocol::CONN),
        "SENSOR" => Ok(Protocol::SENSOR),
        "AVG" => Ok(Protocol::AVG),
        "DISCONN" => Ok(Protocol::DISCONN),
        _ => Err("Invalid protocol".into()),
    }
}

pub struct ConnMsg {
    pub uid: String,
}

impl ConnMsg {
    pub fn from_msg(msg: &String) -> Result<Self, Box<dyn Error>> {
        let parts: Vec<&str> = msg.split("#").collect();

        if parts.len() != 2 {
            error!(
                "Invalid CONN message length: {:?} instead of 2",
                parts.len()
            );
            return Err("Invalid message".into());
        }

        // protocol part
        if parts[0] != "CONN" {
            error!(
                "Invalid CONN protocol header: {:?} instead of CONN",
                parts[0]
            );
            return Err("Invalid protocol".into());
        }

        let id = parts[1].parse::<String>()?;
        if id.len() != 36 {
            error!("Invalid uuid: {:?}", id);
            return Err("Invalid id".into());
        }

        Ok(Self { uid: id })
    }
}

pub struct SensorMsg {
    pub uid: String,
    pub data: f64,
    pub timestamp: i64,
}

impl SensorMsg {
    pub fn from_msg(msg: &String) -> Result<Self, Box<dyn Error>> {
        let parts: Vec<&str> = msg.split("#").collect();

        if parts.len() != 4 {
            error!(
                "Invalid SENSOR message length: {:?} instead of 4",
                parts.len()
            );
            return Err("Invalid message".into());
        }

        // protocol part
        if parts[0] != "SENSOR" {
            error!("Invalid SENSOR header: {:?} instead od SENSOR", parts[0]);
            return Err("Invalid protocol".into());
        }

        let id = parts[1].parse::<String>()?;
        if id.len() != 36 {
            error!("Invalid uuid: {:?}", id);
            return Err("Invalid id".into());
        }

        let timestamp = parts[2].parse::<i64>()?;

        let data = parts[3].parse::<f64>()?;

        Ok(Self {
            uid: id,
            data,
            timestamp,
        })
    }
}

pub struct AvgMsg {
    pub data: f64,
    pub timestamp: i64,
}

impl AvgMsg {
    pub fn to_msg(&self) -> String {
        format!("AVG#{}#{}", self.timestamp, self.data)
    }
}

pub async fn avg_msg_service(state: Arc<crate::AppState>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

    let mut ticks = 0;

    loop {
        interval.tick().await;
        ticks += 1;

        let messages = db::get_last_received_messages(&state.pool, 5)
            .await
            .unwrap_or(Vec::new());

        let size = messages.len();
        if size == 0 {
            warn!("AVG service: no messages to process, Tick {}", ticks);
            continue;
        }

        let mut avg: f64 = 0.0;
        for msg in messages {
            avg += msg.data;
        }
        avg /= size as f64;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let avg_msg = AvgMsg {
            data: avg,
            timestamp: now,
        };

        if db::add_queued_message(&state.pool, avg_msg.to_msg())
            .await
            .is_err()
        {
            error!("AVG service: failed to add message to queue");
        }
    }
}
