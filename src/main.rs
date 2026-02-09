use chrono::{Datelike, Timelike};
use clap::Parser;
use io_gate::comm;
use io_gate::config::Config;
use io_gate::homeassistant::{self, discovery, HomeAssistant};
use io_gate::message::{args::InfoCode, args::OutputState, Message};
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, default_value = "config.yaml")]
    config_path: String,

    // USB Connection
    #[arg(long, default_value = "/dev/ttyACM0")]
    port_name: String,
    #[arg(long, default_value_t = 115200)]
    baud_rate: u32,

    // MQTT connection
    #[arg(long)]
    mqtt_host: String,
    #[arg(long, default_value_t = 1883)]
    mqtt_port: u16,
    #[arg(long, default_value = "")]
    mqtt_username: String,
    #[arg(long, default_value = "")]
    mqtt_password: String,

    // Other
    #[arg(long, default_value = "io-gate")]
    device_name: String,
}

fn init_log() {
    let timer = fmt::time::ChronoLocal::new("%H:%M:%S%.3f".to_string());

    // Configure a custom event formatter
    let format = fmt::format()
        .with_level(true)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_source_location(true)
        .with_timer(timer)
        .compact();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env()
        .expect("RUST_LOG configuration is valid")
        .add_directive("rumqttc=info".parse().unwrap());

    fmt().event_format(format).with_env_filter(filter).init();
}

/// Perform initial configuration and device discovery.
async fn init_config(config: &Config, ha: &HomeAssistant) -> anyhow::Result<()> {
    for (device_name, cfg) in &config.devices {
        let message = discovery::new_device(device_name, cfg);

        // Subscribe to HomeAssistant state changes.
        for component in message.components.values() {
            ha.send(homeassistant::Outgoing::Subscribe(
                component.command_topic.clone(),
            ))
            .await?;
        }

        // Send discovery message to register/update device in HA.
        ha.send(homeassistant::Outgoing::DiscoveryDevice(message))
            .await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_log();
    let args = Args::parse();

    let config = Config::from_file(&args.config_path)?;
    info!("Starting IO Gate. Args: {:?} Config: {:?}", args, config);

    let mut ha_init = homeassistant::Initiator::new(
        "io-gate-mqtt",
        &args.mqtt_host,
        args.mqtt_port,
        &args.mqtt_username,
        &args.mqtt_password,
    )
    .await;
    ha_init.set_topics("iogatetest", "iogatepub").await;
    let ha = Arc::new(ha_init.start().await);

    ha.send(homeassistant::Outgoing::Initial)
        .await
        .expect("Should send");

    init_config(&config, &ha).await?;

    let mut comm = comm::run(args.port_name, args.baud_rate).await?;

    info!("io-gate initialized.");

    // USB -> MQTT
    let ha_sender = ha.clone();
    let task_usb_to_mqtt = async move {
        loop {
            let raw = if let Some(raw) = comm.rx.recv().await {
                raw
            } else {
                // The other end died.
                break;
            };
            let msg = Message::from_raw(&raw);
            let msg = if let Ok(msg) = msg {
                msg
            } else {
                error!("{:?}", msg);
                continue;
            };
            let (device_addr, _) = raw.addr_type();

            info!("CAN->RX: Message {:?}", msg);
            // TODO: Push state messages
            match msg {
                Message::OutputChanged { output, state } => {
                    let on = if let Ok(on) = state.try_to_bool() {
                        on
                    } else {
                        error!("Triggering (switching current) state is not supported.");
                        continue;
                    };

                    let result = ha_sender
                        .send(homeassistant::Outgoing::OutputChanged {
                            device: device_addr,
                            output,
                            on,
                        })
                        .await;
                    if result.is_err() {
                        // The other end died.
                        break;
                    }
                }
                Message::Info { code, arg } => {
                    if code == InfoCode::Started.to_bytes() {
                        info!("Device just started: {}", device_addr);
                        // TODO: Cleanup device state?
                    } else {
                        info!(
                            code = code,
                            arg = arg,
                            "Got unhandled info message from {}",
                            device_addr
                        );
                    }
                }
                _ => {
                    info!("Unhandled incoming message, ignoring: {:?}", msg);
                    continue;
                }
            }
        }
        info!("USB->MQTT task finishing");
        Err::<(), ()>(())
    };

    // MQTT -> USB
    let comm_tx = comm.tx.clone();
    let task_mqtt_to_usb = async move {
        loop {
            let msg = if let Some(msg) = ha.recv().await {
                msg
            } else {
                // The other side died.
                break;
            };

            match msg {
                homeassistant::Incoming::RawTest(_vec) => {
                    info!("Raw test message received");
                }
                homeassistant::Incoming::SetOutput { device, output, on } => {
                    let msg = Message::SetOutput {
                        output,
                        state: OutputState::from_bool(on),
                    };
                    let addr = device;
                    let raw = msg.to_raw(addr);
                    info!("Sending output change request over USB {:?}", raw);
                    if comm_tx.send(raw).await.is_err() {
                        // The other side died.
                        break;
                    }
                }
            }
        }
        // Return Err to break try_join
        info!("MQTT->USB task finishing");
        Err::<(), ()>(())
    };

    let comm_tx = comm.tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;

            let now = chrono::offset::Local::now();
            let msg = Message::TimeAnnouncement {
                year: now.year() as u16,
                month: now.month() as u8,
                day: now.day() as u8,
                hour: now.hour() as u8,
                minute: now.minute() as u8,
                second: now.second() as u8,
                day_of_week: now.weekday().number_from_monday() as u8,
            };
            let addr = 0;
            let raw = msg.to_raw(addr);
            info!("Broadcasting current time {:?}", msg);
            if comm_tx.send(raw).await.is_err() {
                // Transmitter died.
                break;
            }
        }
    });

    // Wait for tasks. If any side dies (comm.reader, comm.writer) this should
    // close the program.
    let _ = tokio::try_join!(task_mqtt_to_usb, task_usb_to_mqtt);
    Ok(())
}
