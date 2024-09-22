all:
	
clean:
	cargo clean
e-paper:
	MCU=esp32c6 cargo build --target riscv32imac-esp-espidf --example e-paper --release
r-e-paper:
	MCU=esp32c6 cargo espflash flash --target riscv32imac-esp-espidf --example e-paper --monitor --release
