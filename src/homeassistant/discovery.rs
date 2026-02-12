use crate::{config, consts};
use serde::Serialize;
use serde_json;
use std::collections::{HashMap, HashSet};
use tracing::error;

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
    pub device_class: Option<String>,
    // pub icon: String,
    pub unique_id: String,
    pub command_topic: Option<String>,
    pub state_topic: String,
}

impl Component {
    pub fn new_switch(name: &str, device_addr: u8, idx: u8, device_class: Option<String>) -> Self {
        Self {
            name: name.to_string(),
            platform: "switch".to_string(),
            device_class: Some(device_class.unwrap_or("switch".to_string())),
            // icon: "mdi:light".to_string(),
            unique_id: format!("io-gate-{}-{}", device_addr, idx),
            command_topic: Some(format!(
                "{}/{}/switch/{}/set",
                consts::HA_CONTROL_TOPIC,
                device_addr,
                idx
            )),
            state_topic: format!(
                "{}/{}/switch/{}/state",
                consts::HA_CONTROL_TOPIC,
                device_addr,
                idx
            ),
        }
    }

    pub fn new_input(name: &str, device_addr: u8, idx: u8, device_class: Option<String>) -> Self {
        // Class list: https://www.home-assistant.io/integrations/binary_sensor/
        Self {
            name: name.to_string(),
            platform: "binary_sensor".to_string(),
            device_class: device_class,
            // icon: "mdi:light".to_string(),
            unique_id: format!("io-gate-in-{}-{}", device_addr, idx),
            command_topic: None,
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

    // Check for duplicates.
    let mut unique_label: HashSet<String> = HashSet::new();
    let mut unique_id: HashSet<u8> = HashSet::new();

    for (label, io) in config.outputs.iter() {
        if unique_label.contains(label) {
            error!("Duplicated output label {} in device {:?}", label, name);
            continue;
        }

        if unique_id.contains(&io.id) {
            error!("Duplicated output IO id {} in device {:?}", io.id, name);
            continue;
        }

        unique_label.insert(label.clone());
        unique_id.insert(io.id);

        // Create device components.
        let component = Component::new_switch(&label, config.addr, io.id, None);
        components.insert(label.clone(), component);
    }

    // Input/Output IDs can collide. Should labels though?
    unique_id.clear();

    for (label, io) in config.inputs.iter() {
        if unique_label.contains(label) {
            error!("Duplicated input/output label {} in device {:?}", label, name);
            continue;
        }

        if unique_id.contains(&io.id) {
            error!("Duplicated input IO id {} in device {:?}", io.id, name);
            continue;
        }

        // Create device components.
        let component = Component::new_input(&label, config.addr, io.id, None);
        components.insert(label.clone(), component);
    }

    Discovery {
        origin,
        device: device_id,
        components,
    }
}
