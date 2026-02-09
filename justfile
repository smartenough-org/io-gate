# justfile mostly for completion with io-ctrl.
# Can overwrite passwords in CLI. just mqtt_username=... run
mqtt_username := "smartenough"
mqtt_password := "smartenough321"
mqtt_host := "ha.ptr.tf"
port_name := "/dev/ttyACM?"


build:
    cargo build

run:
    echo {{port_name}}
    cargo run -- --mqtt-host {{ mqtt_host }} --mqtt-username {{ mqtt_username }} --mqtt-password {{mqtt_password}} --port-name {{ port_name }}

build-release:
    cargo build

run-release:
    cargo run

clippy:
    cargo clippy

format:
    cargo fmt

# mode: makefile
