(https://classroom.github.com/assets/deadline-readme-button-22041afd0340ce965d47ae6ef1cefeee28c7c493a6346c4f15d667ab976d596c.svg)](https://classroom.github.com/a/eG_xYHDU)

# Accessible Navigation Assistant for the Visually Impaired

## Introduction
The Accessible Navigation Assistant is a wearable device designed to help visually impaired individuals navigate their environment safely. Using ultrasonic sensors and haptic feedback, it detects obstacles and provides intuitive, real-time alerts.

## Features
- Dual ultrasonic obstacle detection (left/right)
- Progressive haptic feedback (vibration motors)
- Audio alerts for critical proximity
- Real-time, responsive operation
- Easy prototyping with breadboard power supply

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

### Hardware Assembly
- Connect the ultrasonic sensors, vibration motors, and buzzer to the Raspberry Pi Pico W as per the schematic.
- Power the system using the breadboard power supply module.
- (Schematic and assembly guide: _coming soon_)

### Software Setup
- Install the [Rust toolchain](https://www.rust-lang.org/tools/install).
- Clone this repository.
- Build and flash the firmware to the Pico W:
  ```sh
  cargo build --release
  # Use your preferred method to flash the .uf2 or binary to the Pico W
  ```
- (More detailed instructions: _coming soon_)

## Code Structure
- `src/main.rs`: Main application logic, sensor reading, feedback control
- `src/irqs.rs`: Interrupt handling (if applicable)
- `Cargo.toml`: Project dependencies

## Planned Features / Roadmap
- WiFi connectivity and web configuration
- Smartphone integration
- Machine learning for advanced obstacle classification
- Additional sensor support

## Contributing
We welcome contributions! Please open issues or pull requests to help improve the project