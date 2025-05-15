#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_time::{Duration, Timer, Instant};
use {defmt_rtt as _, panic_probe as _};
use defmt::*;

// Import interrupts definition module
mod irqs;

// Distance state for filtering
struct DistanceState {
    prev_left: f32,
    prev_right: f32,
}

// Ultrasonic sensor structure definition
struct UltrasonicSensor<'d> {
    trigger: Output<'d>,
    echo: Input<'d>,
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Get a handle to the RP's peripherals
    let p = embassy_rp::init(Default::default());
    
    // First ultrasonic sensor (left)
    let trigger_left = Output::new(p.PIN_14, Level::Low);
    let echo_left = Input::new(p.PIN_15, Pull::None);
    
    // Second ultrasonic sensor (right)
    let trigger_right = Output::new(p.PIN_16, Level::Low);
    let echo_right = Input::new(p.PIN_17, Pull::None);
    
    // Buzzer
    let mut buzzer = Output::new(p.PIN_18, Level::Low);
    
    // Create ultrasonic sensor handlers
    let mut ultrasonic_left = UltrasonicSensor {
        trigger: trigger_left,
        echo: echo_left,
    };
    
    let mut ultrasonic_right = UltrasonicSensor {
        trigger: trigger_right,
        echo: echo_right,
    };
    
    // Initialize distance state for filtering
    let mut distance_state = DistanceState {
        prev_left: 100.0,
        prev_right: 100.0,
    };
    
    info!("Starting VisionAssist with dual sensors and advanced audio feedback!");
    
    // Main loop
    loop {
        // Get stable left distance reading
        let raw_left = get_stable_distance(&mut ultrasonic_left).await;
        let left_distance = filter_distance(raw_left, distance_state.prev_left);
        distance_state.prev_left = left_distance;
        
        // Get stable right distance reading
        let raw_right = get_stable_distance(&mut ultrasonic_right).await;
        let right_distance = filter_distance(raw_right, distance_state.prev_right);
        distance_state.prev_right = right_distance;
        
        // Log distances
        info!("Left: {} cm | Right: {} cm", left_distance as u32, right_distance as u32);
        
        // Provide advanced audio feedback
        provide_advanced_audio_feedback(&mut buzzer, left_distance, right_distance).await;
        
        // Small delay before next measurement cycle
        Timer::after(Duration::from_millis(100)).await;
    }
}

// Ultrasonic sensor implementation
impl<'d> UltrasonicSensor<'d> {
    async fn measure_distance(&mut self) -> Result<f32, &'static str> {
        // Send trigger pulse
        self.trigger.set_high();
        Timer::after(Duration::from_micros(10)).await;  // 10µs trigger pulse
        self.trigger.set_low();
        
        // Wait for echo to go high (with timeout)
        let mut timeout = false;
        let timeout_duration = Duration::from_millis(100); // 100ms timeout
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
        
        // Record the time when echo goes high
        let pulse_start = Instant::now();
        
        // Wait for echo to go low (with timeout)
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
        
        // Calculate pulse duration
        let pulse_duration = pulse_start.elapsed();
        
        // Calculate distance (speed of sound = 343m/s = 34300cm/s)
        // Distance = (time × speed of sound) ÷ 2
        let distance_cm = (pulse_duration.as_micros() as f32) * 0.034 / 2.0;
        
        // Clip to reasonable range (2cm to 400cm)
        if distance_cm < 2.0 || distance_cm > 400.0 {
            return Err("Distance out of reasonable range");
        }
        
        Ok(distance_cm)
    }
}

// Function to get stable distance readings by taking multiple measurements
async fn get_stable_distance(sensor: &mut UltrasonicSensor<'_>) -> f32 {
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
                // Just skip invalid readings
            }
        }
        Timer::after(Duration::from_millis(10)).await;
    }
    
    if valid_readings > 0 {
        // Return average of valid readings
        sum / (valid_readings as f32)
    } else {
        // Return "far" if no valid readings
        100.0
    }
}

// Simple low-pass filter to smooth distance readings
fn filter_distance(current: f32, previous: f32) -> f32 {
    // Apply more weight to current reading (70%) and less to previous (30%)
    current * 0.7 + previous * 0.3
}

// Provide audio feedback based on distances from both sensors
async fn provide_advanced_audio_feedback(
    buzzer: &mut Output<'_>,
    left_distance: f32,
    right_distance: f32,
) {
    // Determine left/right danger zones
    let left_danger = if left_distance < 60.0 { 
        Some(categorize_distance(left_distance)) 
    } else { 
        None 
    };
    
    let right_danger = if right_distance < 60.0 { 
        Some(categorize_distance(right_distance)) 
    } else { 
        None 
    };
    
    // If both sides detect obstacles
    if left_danger.is_some() && right_danger.is_some() {
        // Compare danger levels and prioritize higher danger
        let left_level = left_danger.unwrap();
        let right_level = right_danger.unwrap();
        
        if left_level > right_level {
            // Left side is more dangerous
            play_left_pattern(buzzer, left_level).await;
        } else if right_level > left_level {
            // Right side is more dangerous
            play_right_pattern(buzzer, right_level).await;
        } else {
            // Equal danger - alert for both sides
            play_center_pattern(buzzer, left_level).await;
        }
    } 
    // Only left side detects an obstacle
    else if left_danger.is_some() {
        play_left_pattern(buzzer, left_danger.unwrap()).await;
    }
    // Only right side detects an obstacle
    else if right_danger.is_some() {
        play_right_pattern(buzzer, right_danger.unwrap()).await;
    }
    // No obstacles detected
    else {
        // No beep
        buzzer.set_low();
        Timer::after(Duration::from_millis(100)).await;
    }
}

// Helper function to categorize distance into danger levels
fn categorize_distance(distance: f32) -> u8 {
    if distance < 15.0 {
        3 // Highest danger
    } else if distance < 30.0 {
        2 // Medium danger
    } else {
        1 // Low danger
    }
}

// Different patterns for left obstacles
async fn play_left_pattern(buzzer: &mut Output<'_>, danger_level: u8) {
    match danger_level {
        3 => { // Highest danger - very rapid beeps
            for _ in 0..4 {
                buzzer.set_high();
                Timer::after(Duration::from_millis(30)).await;
                buzzer.set_low();
                Timer::after(Duration::from_millis(30)).await;
            }
        },
        2 => { // Medium danger - medium beeps
            for _ in 0..2 {
                buzzer.set_high();
                Timer::after(Duration::from_millis(70)).await;
                buzzer.set_low();
                Timer::after(Duration::from_millis(70)).await;
            }
        },
        _ => { // Low danger - single beep
            buzzer.set_high();
            Timer::after(Duration::from_millis(50)).await;
            buzzer.set_low();
            Timer::after(Duration::from_millis(150)).await;
        }
    }
}

// Different patterns for right obstacles
async fn play_right_pattern(buzzer: &mut Output<'_>, danger_level: u8) {
    match danger_level {
        3 => { // Highest danger - long then short
            buzzer.set_high();
            Timer::after(Duration::from_millis(200)).await;
            buzzer.set_low();
            Timer::after(Duration::from_millis(50)).await;
            buzzer.set_high();
            Timer::after(Duration::from_millis(50)).await;
            buzzer.set_low();
        },
        2 => { // Medium danger - medium burst
            buzzer.set_high();
            Timer::after(Duration::from_millis(150)).await;
            buzzer.set_low();
            Timer::after(Duration::from_millis(50)).await;
        },
        _ => { // Low danger - single long beep
            buzzer.set_high();
            Timer::after(Duration::from_millis(100)).await;
            buzzer.set_low();
            Timer::after(Duration::from_millis(100)).await;
        }
    }
}

// Pattern for when both sides have equal danger
async fn play_center_pattern(buzzer: &mut Output<'_>, danger_level: u8) {
    // Center danger pattern (alternating short-long)
    for _ in 0..danger_level {
        buzzer.set_high();
        Timer::after(Duration::from_millis(50)).await;
        buzzer.set_low();
        Timer::after(Duration::from_millis(50)).await;
        buzzer.set_high();
        Timer::after(Duration::from_millis(100)).await;
        buzzer.set_low();
        Timer::after(Duration::from_millis(50)).await;
    }
}