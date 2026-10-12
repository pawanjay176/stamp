# Explicit shortcuts for the original ESP32-C3 X4. No device action by default.
.DEFAULT_GOAL := help

TARGET := riscv32imc-unknown-none-elf
ELF := target/$(TARGET)/release/stamp-firmware
IMAGE := $(ELF).bin
ESPTOOL := uv tool run --from esptool==4.12.0 esptool.py
PYTHON_WITH_SERIAL := uv tool run --from esptool==4.12.0 python

# Override with: make logs PORT=/dev/cu.usbmodem2101
PORT ?=
# Selected OTA slot verified on this X4. Recheck after browser/OTA updates.
FLASH_OFFSET ?= 0x650000

.PHONY: help build image flash verify logs ports

help:
	@echo 'make build   - compile the release ELF (no device access)'
	@echo 'make image   - build and generate the application .bin'
	@echo 'make flash   - build/image, then WRITE the selected application slot'
	@echo 'make verify  - compare the existing .bin with flash (reads/resets device)'
	@echo 'make logs    - open serial monitor; exit with Ctrl + ]'
	@echo 'make ports   - list candidate serial ports'
	@echo 'PORT is auto-detected when exactly one device is present.'
	@echo 'Override: make logs PORT=/dev/cu.usbmodem2101'
	@echo 'Flash offset: $(FLASH_OFFSET) (must match selected boot slot).'
	@echo 'Use make -n TARGET to inspect commands without running them.'

build:
	cargo build --locked -p stamp-firmware --bin stamp-firmware --features device --target $(TARGET) --release

image: build
	$(ESPTOOL) --chip esp32c3 elf2image --flash_size 16MB -o $(IMAGE) $(ELF)

flash: image
	port=$$(python3 scripts/serial_port.py "$(PORT)") && \
		echo "Writing $(IMAGE) to $$port at $(FLASH_OFFSET)" && \
		$(ESPTOOL) --chip esp32c3 --port "$$port" --baud 460800 write_flash $(FLASH_OFFSET) $(IMAGE)

# Do not rebuild here: verify the exact image file used by the last flash.
verify:
	port=$$(python3 scripts/serial_port.py "$(PORT)") && \
		$(ESPTOOL) --chip esp32c3 --port "$$port" --baud 460800 verify_flash $(FLASH_OFFSET) $(IMAGE)

logs:
	port=$$(python3 scripts/serial_port.py "$(PORT)") && \
		$(PYTHON_WITH_SERIAL) -m serial.tools.miniterm "$$port" 115200 --dtr 0 --rts 0

ports:
	python3 scripts/serial_port.py --list
