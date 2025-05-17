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

// Define distance thresholds for improved real-world usability
const CRITICAL_DISTANCE: f32 = 30.0;  // Very close - requires immediate attention
const WARNING_DISTANCE: f32 = 60.0;   // Warning zone - needs awareness
const NOTICE_DISTANCE: f32 = 100.0;   // Notice zone - gentle feedback

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Get a handle to the RP's peripherals
    let p = embassy_rp::init(Default::default());
    
    // First ultrasonic sensor (left)
    let trigger_left = Output::new(p.PIN_14, Level::Low);
    let echo_left = Input::new(p.PIN_15, Pull::None);
    
    // Second ultrasonic sensor (right)
    let trigger_right = Output::new(p.PIN_16, Level::Low);
    let echo_right = Input::new(p.PIN_17, Pull::None);
    
    // Buzzer - only used for critical warnings
    let mut buzzer = Output::new(p.PIN_18, Level::Low);
    
    // Vibrator motors - primary feedback mechanism
    let mut vibration_left = Output::new(p.PIN_19, Level::Low);
    let mut vibration_right = Output::new(p.PIN_20, Level::Low);

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
    
    info!("Starting VisionAssist with progressive haptic feedback!");
    
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
        
        // Provide progressive haptic and selective audio feedback
        provide_feedback(
            &mut buzzer, 
            &mut vibration_left, 
            &mut vibration_right, 
            left_distance, 
            right_distance
        ).await;
        
        // Small delay before next measurement cycle - reduced for faster response
        Timer::after(Duration::from_millis(50)).await;
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

// Enhanced low-pass filter to smooth distance readings
fn filter_distance(current: f32, previous: f32) -> f32 {
    // Apply more weight to current reading (70%) and less to previous (30%)
    // This provides good responsiveness while reducing noise
    current * 0.7 + previous * 0.3
}

// Improved feedback function with very strong warning for extremely close obstacles
async fn provide_feedback(
    buzzer: &mut Output<'_>,
    vibration_left: &mut Output<'_>,
    vibration_right: &mut Output<'_>,
    left_distance: f32,
    right_distance: f32,
) {
    // IMPORTANT: Always explicitly set the motor states at the beginning
    // This ensures both motors start from a known state (off)
    vibration_left.set_low();
    vibration_right.set_low();
    
    // Check for extremely close obstacles first (emergency warning)
    let extreme_danger_threshold = 10.0; // cm
    let extreme_danger = left_distance < extreme_danger_threshold || right_distance < extreme_danger_threshold;
    
    if extreme_danger {
        // Provide an unmistakable emergency warning
        provide_extreme_danger_warning(buzzer, vibration_left, vibration_right).await;
        // After emergency warning, return to prevent normal processing
        return;
    }
    
    // Normal processing continues below if no extreme danger detected
    
    // Process left side feedback
    let left_intensity = if left_distance < NOTICE_DISTANCE {
        calculate_vibration_intensity(left_distance)
    } else {
        0 // No vibration
    };
    
    // Process right side feedback
    let right_intensity = if right_distance < NOTICE_DISTANCE {
        calculate_vibration_intensity(right_distance)
    } else {
        0 // No vibration
    };
    
    // First handle the left side vibration if needed
    if left_intensity > 0 {
        provide_haptic_feedback(vibration_left, left_intensity).await;
        // Explicitly turn off after feedback pattern
        vibration_left.set_low();
    }
    
    // Then handle the right side vibration if needed
    if right_intensity > 0 {
        provide_haptic_feedback(vibration_right, right_intensity).await;
        // Explicitly turn off after feedback pattern
        vibration_right.set_low();
    }
    
    // Only trigger audio warnings for critical distances (under 30cm)
    if left_distance < CRITICAL_DISTANCE || right_distance < CRITICAL_DISTANCE {
        // Determine which side is more critical
        if left_distance < right_distance {
            // Left side is more critical
            provide_warning_sound(buzzer, left_distance).await;
        } else {
            // Right side is more critical
            provide_warning_sound(buzzer, right_distance).await;
        }
    }
    
    // Ensure buzzer is off after feedback
    buzzer.set_low();
}

// New function for extreme danger warning (very close obstacles)
async fn provide_extreme_danger_warning(
    buzzer: &mut Output<'_>,
    vibration_left: &mut Output<'_>,
    vibration_right: &mut Output<'_>,
) {
    // Unmistakable pattern: alternating strong vibrations with urgent sound
    
    // First cycle - buzzer + left motor
    buzzer.set_high();
    vibration_left.set_high();
    Timer::after(Duration::from_millis(150)).await;
    buzzer.set_low();
    vibration_left.set_low();
    Timer::after(Duration::from_millis(50)).await;
    
    // Second cycle - buzzer + right motor
    buzzer.set_high();
    vibration_right.set_high();
    Timer::after(Duration::from_millis(150)).await;
    buzzer.set_low();
    vibration_right.set_low();
    Timer::after(Duration::from_millis(50)).await;
    
    // Third cycle - buzzer + both motors (strongest warning)
    buzzer.set_high();
    vibration_left.set_high();
    vibration_right.set_high();
    Timer::after(Duration::from_millis(300)).await;
    buzzer.set_low();
    vibration_left.set_low();
    vibration_right.set_low();
    
    // Short pause before returning to normal operation
    Timer::after(Duration::from_millis(100)).await;
}

// Calculate vibration intensity on a more granular scale (0-10)
fn calculate_vibration_intensity(distance: f32) -> u8 {
    if distance < CRITICAL_DISTANCE {
        // Critical zone - strongest vibration (levels 7-10)
        let critical_range = CRITICAL_DISTANCE;
        let normalized = (critical_range - distance.min(critical_range)) / critical_range;
        let level = 7.0 + normalized * 3.0;
        level as u8  // Simple truncation instead of rounding
    } else if distance < WARNING_DISTANCE {
        // Warning zone - medium vibration (levels 4-6)
        let warning_range = WARNING_DISTANCE - CRITICAL_DISTANCE;
        let normalized = (WARNING_DISTANCE - distance) / warning_range;
        let level = 4.0 + normalized * 2.0;
        level as u8
    } else if distance < NOTICE_DISTANCE {
        // Notice zone - gentle vibration (levels 1-3)
        let notice_range = NOTICE_DISTANCE - WARNING_DISTANCE;
        let normalized = (NOTICE_DISTANCE - distance) / notice_range;
        let level = 1.0 + normalized * 2.0;
        level as u8
    } else {
        // Beyond notice zone - no vibration
        0
    }
}

// Modified haptic feedback function that properly manages motor state
async fn provide_haptic_feedback(motor: &mut Output<'_>, intensity: u8) {
    match intensity {
        10 => { // Maximum intensity - continuous strong vibration
            motor.set_high();
            Timer::after(Duration::from_millis(80)).await;
            // Don't turn off here - will be turned off by the caller
        },
        9 => { // Very strong vibration - almost continuous with tiny breaks
            motor.set_high();
            Timer::after(Duration::from_millis(80)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(20)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(80)).await;
        },
        8 => { // Strong vibration - brief pattern
            motor.set_high();
            Timer::after(Duration::from_millis(70)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(30)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(70)).await;
        },
        7 => { // Moderate-strong vibration
            motor.set_high();
            Timer::after(Duration::from_millis(60)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(40)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(60)).await;
        },
        6 => { // Moderate vibration
            motor.set_high();
            Timer::after(Duration::from_millis(50)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(50)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(50)).await;
        },
        5 => { // Medium vibration
            motor.set_high();
            Timer::after(Duration::from_millis(40)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(60)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(40)).await;
        },
        4 => { // Light-medium vibration
            motor.set_high();
            Timer::after(Duration::from_millis(30)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(70)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(30)).await;
        },
        3 => { // Light vibration
            motor.set_high();
            Timer::after(Duration::from_millis(20)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(80)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(20)).await;
        },
        2 => { // Very light vibration
            motor.set_high();
            Timer::after(Duration::from_millis(10)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(90)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(10)).await;
        },
        1 => { // Minimal vibration - just enough to notice
            motor.set_high();
            Timer::after(Duration::from_millis(5)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(95)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(5)).await;
        },
        _ => { // No vibration - ensure motor is off
            motor.set_low();
            Timer::after(Duration::from_millis(10)).await;
        }
    }
}

// Provide warning sounds for critical distances
async fn provide_warning_sound(buzzer: &mut Output<'_>, distance: f32) {
    // Only active for distances under CRITICAL_DISTANCE (30cm)
    if distance < 10.0 {
        // Urgent alarm for very close objects (under 10cm)
        // Rapid, high-pitched beeping
        for _ in 0..3 {
            buzzer.set_high();
            Timer::after(Duration::from_millis(25)).await;
            buzzer.set_low();
            Timer::after(Duration::from_millis(25)).await;
        }
    } else if distance < 20.0 {
        // Medium urgency alarm (10-20cm)
        // Medium-paced beeping
        for _ in 0..2 {
            buzzer.set_high();
            Timer::after(Duration::from_millis(50)).await;
            buzzer.set_low();
            Timer::after(Duration::from_millis(50)).await;
        }
    } else {
        // Low urgency alarm (20-30cm)
        // Single beep
        buzzer.set_high();
        Timer::after(Duration::from_millis(70)).await;
        buzzer.set_low();
    }
}