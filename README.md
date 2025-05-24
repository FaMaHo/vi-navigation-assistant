# Accessible Navigation Assistant for the Visually Impaired

## Introduction
The Accessible Navigation Assistant is a wearable device designed to help visually impaired individuals navigate their environment safely. Using ultrasonic sensors and haptic feedback, it detects obstacles and provides intuitive, real-time alerts.

## Features
- Dual ultrasonic obstacle detection (left/right)
- Progressive haptic feedback (vibration motors)
- Audio alerts for critical proximity
- Real-time, responsive operation
- Easy prototyping with breadboard power supply
- WiFi connectivity for remote monitoring and configuration

## Hardware Overview
- Raspberry Pi Pico W (RP2040)
- 2× HC-SR04 Ultrasonic Distance Sensors
- 2× Vibration Motors
- 1× Buzzer
- Breadboard Power Supply Module
- Breadboard, jumper wires, resistors, transistors, diodes, enclosure, straps

## How It Works
The device continuously measures distances to the left and right using ultrasonic sensors. As obstacles get closer, the corresponding vibration motor increases in intensity and pattern. If an object is critically close, the buzzer sounds an alert. This allows users to sense both the direction and urgency of obstacles.

## Getting Started

### Prerequisites
- Rust toolchain (install from https://www.rust-lang.org/tools/install)
- Raspberry Pi Pico W
- Required hardware components (see Hardware Overview)
- Debug probe (optional, for development)

### Hardware Assembly
1. Connect the ultrasonic sensors, vibration motors, and buzzer to the Raspberry Pi Pico W as per the schematic in `ProjectKiCADSchematic/`.
2. Power the system using the breadboard power supply module.
3. For development, connect a debug probe to the Pico W's debug pins.

### Software Setup
1. Clone this repository:
   ```sh
   git clone https://github.com/FaMaHo/vi-navigation-assistant.git
   cd vi-navigation-assistant
   ```

2. Build the project:
   ```sh
   cargo build --release
   ```

3. Flash the firmware:
   - For development with debug probe:
     ```sh
     cargo run
     ```
   - For direct flashing:
     - Put the Pico W in bootloader mode
     - Copy the generated `.uf2` file from `target/thumbv6m-none-eabi/release/` to the mounted Pico W drive

## Project Structure
```
.
├── src/                    # Source code
│   ├── main.rs            # Main application logic
│   └── irqs.rs            # Interrupt handling
├── ProjectKiCADSchematic/ # KiCAD project files
├── firmware/              # Firmware files
├── cyw43-firmware/        # WiFi firmware
├── datasheets/           # Component datasheets
├── embassy-lab-utils/    # Utility functions
├── Cargo.toml            # Project dependencies
└── memory.x              # Memory layout configuration
```

## Development
- The project uses the `embassy` async runtime for efficient task management
- WiFi functionality is implemented using the `cyw43` driver
- Debug probe support is included for development and debugging

## Contributing
We welcome contributions! Please:
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request
