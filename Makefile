all:
	
clean:
	cargo clean
r-mqtt:
	MCU=esp32c6 cargo espflash flash --target riscv32imac-esp-espidf --monitor --example mqtt_client --release
r-wifi:
	MCU=esp32c6 cargo espflash flash --target riscv32imac-esp-espidf --monitor --example wifi --release
e-paper:
	MCU=esp32c6 cargo build --target riscv32imac-esp-espidf --example e-paper --release
r-e-paper:
	MCU=esp32c6 cargo espflash flash --target riscv32imac-esp-espidf --example e-paper --monitor --release
