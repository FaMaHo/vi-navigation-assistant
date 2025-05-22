#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Input, Level, Output, Pull},
};
use embassy_time::{Duration, Timer, Instant};
use defmt::info;
use defmt_rtt as _; // Import defmt RTT logger
use panic_probe as _; // Import panic handler

// for handling interrupts and wifi
mod irqs;
mod tcp_server;
mod web_server;
mod wifi_utils;

// keeping track of previous distances for smoothing
struct DistanceState {
    prev_left: f32,
    prev_right: f32,
}

// basic sensor structure
struct UltrasonicSensor<'d> {
    trigger: Output<'d>,
    echo: Input<'d>,
}

// these might need adjusting after testing
const CRITICAL_DISTANCE: f32 = 30.0;  // very close obstacles
const WARNING_DISTANCE: f32 = 60.0;   // getting closer
const NOTICE_DISTANCE: f32 = 100.0;   // far enough but worth noting

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting VisionAssist with WiFi configuration...");

    // Initialize the RP2040 and keep a reference to the pins we'll need
    let p = embassy_rp::init(Default::default());
    
    // Save the pins we need for our sensors and feedback BEFORE WiFi init
    let pin_14 = p.PIN_14;
    let pin_15 = p.PIN_15;
    let pin_16 = p.PIN_16;
    let pin_17 = p.PIN_17;
    let pin_18 = p.PIN_18;
    let pin_19 = p.PIN_19;
    let pin_20 = p.PIN_20;
    
    // Initialize network stack
    info!("Initializing network stack...");
    let (stack, socket) = wifi_utils::init_network_stack(
        &spawner,
        p.PIN_23,
        p.PIN_24,
        p.PIN_25,
        p.PIN_29,
        p.PIO0,
        p.DMA_CH2,
    ).await;
    info!("Network stack initialized successfully");
    
    // Start TCP server
    spawner.spawn(tcp_server::tcp_server_task(stack, socket)).unwrap();
    
    // Start web server
    spawner.spawn(web_server::web_server_task(stack)).unwrap();
    
    // Now configure our sensor and feedback pins using the pins we saved
    let trigger_left = Output::new(pin_14, Level::Low);
    let echo_left = Input::new(pin_15, Pull::None);
    
    let trigger_right = Output::new(pin_16, Level::Low);
    let echo_right = Input::new(pin_17, Pull::None);
    
    let mut buzzer = Output::new(pin_18, Level::Low);
    
    let mut vibration_left = Output::new(pin_19, Level::Low);
    let mut vibration_right = Output::new(pin_20, Level::Low);

    // Create sensor objects
    let mut ultrasonic_left = UltrasonicSensor {
        trigger: trigger_left,
        echo: echo_left,
    };
    
    let mut ultrasonic_right = UltrasonicSensor {
        trigger: trigger_right,
        echo: echo_right,
    };
    
    // Initial distance state
    let mut distance_state = DistanceState {
        prev_left: 100.0,
        prev_right: 100.0,
    };
    
    info!("Sensors and feedback systems initialized");
    info!("Connect to WiFi AP 'VisionAssist' to configure device");
    info!("TCP server running on port 8080, Web interface on port 80");
    
    // Main loop
    loop {
        // Get left distance
        let raw_left = match get_stable_distance(&mut ultrasonic_left).await {
            Ok(dist) => dist,
            Err(_) => 100.0, // Default safe value on error
        };
        let left_distance = filter_distance(raw_left, distance_state.prev_left);
        distance_state.prev_left = left_distance;
        
        // Get right distance
        let raw_right = match get_stable_distance(&mut ultrasonic_right).await {
            Ok(dist) => dist,
            Err(_) => 100.0, // Default safe value on error
        };
        let right_distance = filter_distance(raw_right, distance_state.prev_right);
        distance_state.prev_right = right_distance;
        
        // Update the shared state for TCP server
        unsafe {
            tcp_server::LEFT_DISTANCE = left_distance;
            tcp_server::RIGHT_DISTANCE = right_distance;
        }
        
        // Log distances for debugging
        info!("Left: {} cm | Right: {} cm", left_distance as u32, right_distance as u32);
        
        // Provide haptic and audio feedback
        provide_feedback(
            &mut buzzer, 
            &mut vibration_left, 
            &mut vibration_right, 
            left_distance, 
            right_distance
        ).await;
        
        // Brief delay between measurements
        Timer::after(Duration::from_millis(50)).await;
    }
}

// Ultrasonic sensor implementation
impl<'d> UltrasonicSensor<'d> {
    async fn measure_distance(&mut self) -> Result<f32, &'static str> {
        // Send trigger pulse
        self.trigger.set_high();
        Timer::after(Duration::from_micros(10)).await;
        self.trigger.set_low();
        
        // wait for echo to start with timeout
        let mut timeout = false;
        let timeout_duration = Duration::from_millis(100);
        let start = Instant::now();
        
        while self.echo.is_low() {
            if start.elapsed() > timeout_duration {
                timeout = true;
                break;
            }
            Timer::after(Duration::from_micros(10)).await;
        }
        
        if timeout {
            return Err("Echo signal timed out (start)");
        }
        
        // start timing when echo goes high
        let pulse_start = Instant::now();
        
        // wait for echo to end
        timeout = false;
        let start = Instant::now();
        
        while self.echo.is_high() {
            if start.elapsed() > timeout_duration {
                timeout = true;
                break;
            }
            Timer::after(Duration::from_micros(10)).await;
        }
        
        if timeout {
            return Err("Echo signal timed out (end)");
        }
        
        // calculate pulse duration
        let pulse_duration = pulse_start.elapsed();
        
        // calculate distance using speed of sound
        let distance_cm = (pulse_duration.as_micros() as f32) * 0.034 / 2.0;
        
        // filter out unreasonable readings
        if distance_cm < 2.0 || distance_cm > 400.0 {
            return Err("Distance out of reasonable range");
        }
        
        Ok(distance_cm)
    }
}

// Get stable distance readings by averaging
async fn get_stable_distance(sensor: &mut UltrasonicSensor<'_>) -> Result<f32, &'static str> {
    let mut valid_readings = 0;
    let mut sum = 0.0;
    
    // Try up to 5 times to get 3 valid readings
    for _ in 0..5 {
        if valid_readings >= 3 {
            break;
        }
        
        match sensor.measure_distance().await {
            Ok(dist) => {
                sum += dist;
                valid_readings += 1;
            },
            Err(_) => {
                // Skip invalid readings
            }
        }
        Timer::after(Duration::from_millis(10)).await;
    }
    
    if valid_readings > 0 {
        // Return average
        Ok(sum / (valid_readings as f32))
    } else {
        // No valid readings
        Err("Failed to get any valid distance readings")
    }
}

// Simple low-pass filter to smooth readings
fn filter_distance(current: f32, previous: f32) -> f32 {
    // Using 70/30 weighting for responsiveness
    current * 0.7 + previous * 0.3
}

// Main feedback function
async fn provide_feedback(
    buzzer: &mut Output<'_>,
    vibration_left: &mut Output<'_>,
    vibration_right: &mut Output<'_>,
    left_distance: f32,
    right_distance: f32,
) {
    // Always start with motors off
    vibration_left.set_low();
    vibration_right.set_low();
    
    // Check for extremely close obstacles
    let extreme_danger_threshold = 10.0; // cm
    let extreme_danger = left_distance < extreme_danger_threshold || right_distance < extreme_danger_threshold;
    
    if extreme_danger {
        // Special warning for very close objects
        provide_extreme_danger_warning(buzzer, vibration_left, vibration_right).await;
        return;
    }
    
    // Left side intensity
    let left_intensity = if left_distance < NOTICE_DISTANCE {
        calculate_vibration_intensity(left_distance)
    } else {
        0 // no vibration
    };
    
    // Right side intensity
    let right_intensity = if right_distance < NOTICE_DISTANCE {
        calculate_vibration_intensity(right_distance)
    } else {
        0 // no vibration
    };
    
    // Apply left vibration
    if left_intensity > 0 {
        provide_haptic_feedback(vibration_left, left_intensity).await;
        vibration_left.set_low();
    }
    
    // Apply right vibration
    if right_intensity > 0 {
        provide_haptic_feedback(vibration_right, right_intensity).await;
        vibration_right.set_low();
    }
    
    // Sound only for close objects
    if left_distance < CRITICAL_DISTANCE || right_distance < CRITICAL_DISTANCE {
        if left_distance < right_distance {
            provide_warning_sound(buzzer, left_distance).await;
        } else {
            provide_warning_sound(buzzer, right_distance).await;
        }
    }
    
    // Ensure buzzer is off
    buzzer.set_low();
}

// Strong warning pattern for very close obstacles
async fn provide_extreme_danger_warning(
    buzzer: &mut Output<'_>,
    vibration_left: &mut Output<'_>,
    vibration_right: &mut Output<'_>,
) {
    // First pattern - left side
    buzzer.set_high();
    vibration_left.set_high();
    Timer::after(Duration::from_millis(150)).await;
    buzzer.set_low();
    vibration_left.set_low();
    Timer::after(Duration::from_millis(50)).await;
    
    // Second pattern - right side
    buzzer.set_high();
    vibration_right.set_high();
    Timer::after(Duration::from_millis(150)).await;
    buzzer.set_low();
    vibration_right.set_low();
    Timer::after(Duration::from_millis(50)).await;
    
    // Third pattern - both sides
    buzzer.set_high();
    vibration_left.set_high();
    vibration_right.set_high();
    Timer::after(Duration::from_millis(300)).await;
    buzzer.set_low();
    vibration_left.set_low();
    vibration_right.set_low();
    
    // Pause before next cycle
    Timer::after(Duration::from_millis(100)).await;
}

// Calculate vibration intensity (0-10 scale)
fn calculate_vibration_intensity(distance: f32) -> u8 {
    if distance < CRITICAL_DISTANCE {
        // Critical zone (levels 7-10)
        let critical_range = CRITICAL_DISTANCE;
        let normalized = (critical_range - distance.min(critical_range)) / critical_range;
        let level = 7.0 + normalized * 3.0;
        level as u8
    } else if distance < WARNING_DISTANCE {
        // Warning zone (levels 4-6)
        let warning_range = WARNING_DISTANCE - CRITICAL_DISTANCE;
        let normalized = (WARNING_DISTANCE - distance) / warning_range;
        let level = 4.0 + normalized * 2.0;
        level as u8
    } else if distance < NOTICE_DISTANCE {
        // Notice zone (levels 1-3)
        let notice_range = NOTICE_DISTANCE - WARNING_DISTANCE;
        let normalized = (NOTICE_DISTANCE - distance) / notice_range;
        let level = 1.0 + normalized * 2.0;
        level as u8
    } else {
        // Beyond notice zone
        0
    }
}

// Haptic feedback patterns for different intensities
async fn provide_haptic_feedback(motor: &mut Output<'_>, intensity: u8) {
    match intensity {
        10 => { // Maximum intensity
            motor.set_high();
            Timer::after(Duration::from_millis(80)).await;
        },
        9 => { // Very strong
            motor.set_high();
            Timer::after(Duration::from_millis(80)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(20)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(80)).await;
        },
        8 => { // Strong
            motor.set_high();
            Timer::after(Duration::from_millis(70)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(30)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(70)).await;
        },
        7 => { // Moderate-strong
            motor.set_high();
            Timer::after(Duration::from_millis(60)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(40)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(60)).await;
        },
        6 => { // Moderate
            motor.set_high();
            Timer::after(Duration::from_millis(50)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(50)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(50)).await;
        },
        5 => { // Medium
            motor.set_high();
            Timer::after(Duration::from_millis(40)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(60)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(40)).await;
        },
        4 => { // Light-medium
            motor.set_high();
            Timer::after(Duration::from_millis(30)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(70)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(30)).await;
        },
        3 => { // Light
            motor.set_high();
            Timer::after(Duration::from_millis(20)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(80)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(20)).await;
        },
        2 => { // Very light
            motor.set_high();
            Timer::after(Duration::from_millis(10)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(90)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(10)).await;
        },
        1 => { // Minimal
            motor.set_high();
            Timer::after(Duration::from_millis(5)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(95)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(5)).await;
        },
        _ => { // No vibration
            motor.set_low();
            Timer::after(Duration::from_millis(10)).await;
        }
    }
}

// Warning sounds for different distance ranges
async fn provide_warning_sound(buzzer: &mut Output<'_>, distance: f32) {
    if distance < 10.0 {
        // Very close - rapid beeping
        for _ in 0..3 {
            buzzer.set_high();
            Timer::after(Duration::from_millis(25)).await;
            buzzer.set_low();
            Timer::after(Duration::from_millis(25)).await;
        }
    } else if distance < 20.0 {
        // Medium close - moderate beeping
        for _ in 0..2 {
            buzzer.set_high();
            Timer::after(Duration::from_millis(50)).await;
            buzzer.set_low();
            Timer::after(Duration::from_millis(50)).await;
        }
    } else {
        // Not as close - single beep
        buzzer.set_high();
        Timer::after(Duration::from_millis(70)).await;
        buzzer.set_low();
    }
}