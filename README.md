# Accessible Navigation Assistant for the Visually Impaired

## Short Description

The Accessible Navigation Assistant is a wearable electronic device designed to enhance spatial awareness for visually impaired individuals. The core function of this system is to detect obstacles in the user's path using ultrasonic sensors and provide real-time haptic feedback through strategically positioned vibration motors.

The device operates on a simple yet effective principle: as the user approaches an obstacle, the vibration intensity increases proportionally to proximity, creating an intuitive understanding of spatial relationships. By incorporating directional feedback (left/right vibration motors), the system allows users to distinguish not only the presence of obstacles but also their relative position.

This device is designed to complement—rather than replace—traditional mobility aids such as white canes. By providing additional environmental information through a different sensory channel, the system aims to improve confidence and safety during navigation, particularly in unfamiliar environments. The wireless capabilities enable remote configuration and usage data collection, allowing for ongoing optimization of the device's performance.

## Motivation

Navigation and spatial awareness present significant challenges for visually impaired individuals, particularly in unfamiliar environments. While traditional mobility aids like white canes provide valuable tactile information about the immediate surroundings, they have limitations in terms of their detection range and ability to convey directional information.

This project aims to address these challenges by creating an affordable, wearable device that provides additional environmental feedback through haptic stimulation. The system is designed with user comfort and intuitive operation in mind, requiring minimal training to use effectively.

The choice of Rust for this project is motivated by its memory safety guarantees without a garbage collector, which makes it ideal for embedded systems with limited resources. Additionally, Rust's growing ecosystem for embedded development, particularly with frameworks like embassy-rs, provides a robust foundation for building reliable devices with predictable performance.

## Architecture

### System Components

The system consists of the following main components:

1. **Central Processing Unit:** Raspberry Pi Pico W (RP2040 with built-in WiFi)
2. **Sensor Module:**
   - Two HC-SR04 Ultrasonic Distance Sensors (left and right)
3. **Feedback Module:**
   - Two Vibration Motors (left and right)
   - Optional small buzzer for audio feedback
4. **Power Management:**
   - USB Power Bank or LiPo Battery with JST connector
5. **Communication:**
   - WiFi connectivity for remote configuration and data collection

### Component Interconnection

The Raspberry Pi Pico W serves as the central hub, collecting distance data from the ultrasonic sensors mounted on the left and right sides of the device. This data is processed to determine the presence and proximity of obstacles. Based on this information, the system activates the corresponding vibration motors with intensity proportional to the detected proximity. The WiFi module enables remote configuration and usage data collection for ongoing optimization.

## Hardware Design

### Components List (Bill of Materials)

- 1× Raspberry Pi Pico W (RP2040 with built-in WiFi)
- 2× HC-SR04 Ultrasonic Distance Sensors
- 2× Vibration Motors
- 1× Small Buzzer (optional for audio feedback)
- 1× Power source (USB Power Bank or LiPo Battery with JST connector)
- 1× Small Breadboard for prototyping
- 1× Set of Jumper Wires
- Various Resistors (330Ω, 470Ω for voltage dividers)
- 2× 2N2222 Transistors (for driving vibration motors)
- 2× 1N4148 Diodes (flyback protection)
- 1× Plastic Enclosure
- 1× Set of Velcro/Elastic Straps for attachment

### Schematic

[A detailed circuit schematic will be added here, created with KiCad EDA]

## Software Design

### Software Architecture

The software architecture follows an event-driven, task-based approach using the embassy-rs async runtime. The main components include:

1. **Sensor Management:**
   - Ultrasonic sensor distance measurement
   - Detection filtering and noise reduction
   - Obstacle position and proximity calculation
   
2. **Feedback Control:**
   - Vibration motor intensity modulation based on proximity
   - Directional feedback management (left/right)
   - Optional audio feedback for critical alerts

3. **Communication:**
   - WiFi connectivity for configuration and data collection
   - Simple web interface for device settings
   - Usage data logging for performance optimization

4. **Power Management:**
   - Sleep modes for energy conservation
   - Battery level monitoring

### Software Bill of Materials

- **embassy-rs:** Framework for async programming on embedded devices
- **embassy-rp:** Hardware abstraction layer for Raspberry Pi Pico
- **embassy-time:** Timing utilities for the Embassy framework
- **embassy-net:** Networking stack for Embassy
- **embassy-net-driver:** Network driver interface
- **cyw43:** Driver for the Pico W's WiFi chip
- **embedded-hal:** Hardware abstraction layer for embedded systems
- **static-cell:** Allocation of static memory
- **heapless:** Data structures that don't require dynamic memory allocation
- **defmt:** Debugging formatted output
- **panic-probe:** Panic implementation for debugging

### Functional Diagram

[A detailed functional diagram will be added here, similar to the architecture diagram in the document]

## Weekly Log

### Week 1 (April 25 - May 1)
- Project proposal submission
- Initial research on ultrasonic sensors and haptic feedback mechanisms
- Set up development environment for Rust embedded programming

[Future weeks will be added as the project progresses]

## References

1. Embassy-rs documentation: https://embassy.dev/
2. Raspberry Pi Pico W datasheet
3. HC-SR04 Ultrasonic sensor documentation
4. Research papers on assistive technology for visually impaired individuals

## Future Enhancements

- Integration with smartphone apps for additional features
- Machine learning for improved obstacle classification
- Additional sensors for enhanced environmental perception (e.g., infrared for transparent obstacles)
- Customizable feedback patterns for different types of obstacles
