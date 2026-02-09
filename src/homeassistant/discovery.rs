use crate::{config, consts};
use serde::Serialize;
use serde_json;
use std::collections::{HashMap, HashSet};
use tracing::warn;

/// Device identifier
#[derive(Serialize, Debug, Default)]
pub struct DeviceId {
    pub name: String,
    /// Device CAN bus ID + CAN Address to uniquely identify device.
    pub identifiers: Vec<String>,
    pub manufacturer: String,
    // hw, sw, ...
}

/// Discovery origin - this software identifier.
#[derive(Serialize, Debug, Default)]
pub struct Origin {
    name: String,
    sw_version: String,
    support_url: String,
}

/// Represents a component - a part of Device defined by Discovery
#[derive(Serialize, Debug)]
pub struct Component {
    pub name: String,
    pub platform: String,
    /// Changes icon; outlet or switch
    pub device_class: String,
    // pub icon: String,
    pub unique_id: String,
    pub command_topic: String,
    pub state_topic: String,
}

impl Component {
    pub fn new_switch(name: &str, device_addr: u8, idx: u8) -> Self {
        Self {
            name: name.to_string(),
            platform: "switch".to_string(),
            device_class: "switch".to_string(),
            // icon: "mdi:light".to_string(),
            unique_id: format!("io-gate-{}-{}", device_addr, idx),
            command_topic: format!(
                "{}/{}/switch/{}/set",
                consts::HA_CONTROL_TOPIC,
                device_addr,
                idx
            ),
            state_topic: format!(
                "{}/{}/switch/{}/state",
                consts::HA_CONTROL_TOPIC,
                device_addr,
                idx
            ),
        }
    }
}

// config topic: homeassistant/binary_sensor/garden/config
// <discovery_prefix>/<component>/[<node_id>/]<object_id>/config
// component == switch, node_id == omit, object_id == unique_id
// If using device discovery then component is == `device`.
#[derive(Serialize, Debug)]
pub struct Discovery {
    pub device: DeviceId,
    pub origin: Origin,

    pub components: HashMap<String, Component>,
}

impl Discovery {
    pub fn serialize(&self) -> String {
        serde_json::to_string(self).expect("All should be serializable")
    }
}

pub fn new_device(name: &str, config: &config::DeviceConfig) -> Discovery {
    let origin = Origin {
        name: consts::GATE_NAME.to_string(),
        sw_version: consts::GATE_VERSION.to_string(),
        support_url: consts::GATE_URL.to_string(),
    };

    let device_id = DeviceId {
        name: name.to_string(),
        identifiers: vec![format!("gate-{}", config.addr)],
        manufacturer: "smartenough".to_string(),
    };

    let mut components = HashMap::new();

    let unique: HashSet<String> = config.outputs.labels.iter().cloned().collect();
    if unique.len() != config.outputs.labels.len() {
        warn!("Duplicated labels in list: {:?}", config.outputs.labels);
    }

    for i in 0..config.outputs.count {
        let name = if let Some(label) = config.outputs.labels.get(i as usize) {
            label.to_string()
        } else {
            format!("{}-{}", name, i)
        };
        let component = Component::new_switch(&name, config.addr, i);

        components.insert(name, component);
    }

    Discovery {
        origin,
        device: device_id,
        components,
    }
}
