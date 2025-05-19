#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_time::{Duration, Timer, Instant};
use {defmt_rtt as _, panic_probe as _};
use defmt::*;

// for handling interrupts
mod irqs;

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
async fn main(_spawner: Spawner) {
    // initialize RP2040 
    let p = embassy_rp::init(Default::default());
    
    // left sensor pins
    let trigger_left = Output::new(p.PIN_14, Level::Low);
    let echo_left = Input::new(p.PIN_15, Pull::None);
    
    // right sensor pins
    let trigger_right = Output::new(p.PIN_16, Level::Low);
    let echo_right = Input::new(p.PIN_17, Pull::None);
    
    // buzzer for audio feedback
    let mut buzzer = Output::new(p.PIN_18, Level::Low);
    
    // vibration motors
    let mut vibration_left = Output::new(p.PIN_19, Level::Low);
    let mut vibration_right = Output::new(p.PIN_20, Level::Low);

    // create sensor objects
    let mut ultrasonic_left = UltrasonicSensor {
        trigger: trigger_left,
        echo: echo_left,
    };
    
    let mut ultrasonic_right = UltrasonicSensor {
        trigger: trigger_right,
        echo: echo_right,
    };
    
    // initial distance state
    let mut distance_state = DistanceState {
        prev_left: 100.0,
        prev_right: 100.0,
    };
    
    info!("Starting VisionAssist with progressive haptic feedback!");
    
    // main loop
    loop {
        // get left distance
        let raw_left = get_stable_distance(&mut ultrasonic_left).await;
        let left_distance = filter_distance(raw_left, distance_state.prev_left);
        distance_state.prev_left = left_distance;
        
        // get right distance
        let raw_right = get_stable_distance(&mut ultrasonic_right).await;
        let right_distance = filter_distance(raw_right, distance_state.prev_right);
        distance_state.prev_right = right_distance;
        
        // for debugging - should remove before demo
        info!("Left: {} cm | Right: {} cm", left_distance as u32, right_distance as u32);
        
        // provide feedback based on distances
        provide_feedback(
            &mut buzzer, 
            &mut vibration_left, 
            &mut vibration_right, 
            left_distance, 
            right_distance
        ).await;
        
        // reduced delay from 200ms for better responsiveness
        Timer::after(Duration::from_millis(50)).await;
    }
}

// ultrasonic sensor implementation
impl<'d> UltrasonicSensor<'d> {
    async fn measure_distance(&mut self) -> Result<f32, &'static str> {
        // Send trigger pulse
        self.trigger.set_high();
        Timer::after(Duration::from_micros(10)).await;
        self.trigger.set_low();
        
        // wait for echo to start with timeout
        let mut timeout = false;
        let timeout_duration = Duration::from_millis(100); // increased from 50ms
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

// get more stable readings by averaging
async fn get_stable_distance(sensor: &mut UltrasonicSensor<'_>) -> f32 {
    let mut valid_readings = 0;
    let mut sum = 0.0;
    
    // try up to 5 times to get 3 valid readings
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
                // skip invalid readings
            }
        }
        Timer::after(Duration::from_millis(10)).await;
    }
    
    if valid_readings > 0 {
        // return average
        sum / (valid_readings as f32)
    } else {
        // no valid readings
        100.0
    }
}

// simple low-pass filter to smooth readings
fn filter_distance(current: f32, previous: f32) -> f32 {
    // using 70/30 weighting for responsiveness
    current * 0.7 + previous * 0.3
}

// main feedback function
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
    
    // check for extremely close obstacles
    let extreme_danger_threshold = 10.0; // cm
    let extreme_danger = left_distance < extreme_danger_threshold || right_distance < extreme_danger_threshold;
    
    if extreme_danger {
        // special warning for very close objects
        provide_extreme_danger_warning(buzzer, vibration_left, vibration_right).await;
        return;
    }
    
    // left side intensity
    let left_intensity = if left_distance < NOTICE_DISTANCE {
        calculate_vibration_intensity(left_distance)
    } else {
        0 // no vibration
    };
    
    // right side intensity
    let right_intensity = if right_distance < NOTICE_DISTANCE {
        calculate_vibration_intensity(right_distance)
    } else {
        0 // no vibration
    };
    
    // apply left vibration
    if left_intensity > 0 {
        provide_haptic_feedback(vibration_left, left_intensity).await;
        vibration_left.set_low();
    }
    
    // apply right vibration
    if right_intensity > 0 {
        provide_haptic_feedback(vibration_right, right_intensity).await;
        vibration_right.set_low();
    }
    
    // sound only for close objects
    if left_distance < CRITICAL_DISTANCE || right_distance < CRITICAL_DISTANCE {
        if left_distance < right_distance {
            provide_warning_sound(buzzer, left_distance).await;
        } else {
            provide_warning_sound(buzzer, right_distance).await;
        }
    }
    
    // ensure buzzer is off
    buzzer.set_low();
}

// strong warning pattern for very close obstacles
async fn provide_extreme_danger_warning(
    buzzer: &mut Output<'_>,
    vibration_left: &mut Output<'_>,
    vibration_right: &mut Output<'_>,
) {
    // first pattern - left side
    buzzer.set_high();
    vibration_left.set_high();
    Timer::after(Duration::from_millis(150)).await;
    buzzer.set_low();
    vibration_left.set_low();
    Timer::after(Duration::from_millis(50)).await;
    
    // second pattern - right side
    buzzer.set_high();
    vibration_right.set_high();
    Timer::after(Duration::from_millis(150)).await;
    buzzer.set_low();
    vibration_right.set_low();
    Timer::after(Duration::from_millis(50)).await;
    
    // third pattern - both sides
    buzzer.set_high();
    vibration_left.set_high();
    vibration_right.set_high();
    Timer::after(Duration::from_millis(300)).await;
    buzzer.set_low();
    vibration_left.set_low();
    vibration_right.set_low();
    
    // pause before next cycle
    Timer::after(Duration::from_millis(100)).await;
}

// calculate vibration intensity (0-10 scale)
fn calculate_vibration_intensity(distance: f32) -> u8 {
    if distance < CRITICAL_DISTANCE {
        // critical zone (levels 7-10)
        let critical_range = CRITICAL_DISTANCE;
        let normalized = (critical_range - distance.min(critical_range)) / critical_range;
        let level = 7.0 + normalized * 3.0;
        level as u8
    } else if distance < WARNING_DISTANCE {
        // warning zone (levels 4-6)
        let warning_range = WARNING_DISTANCE - CRITICAL_DISTANCE;
        let normalized = (WARNING_DISTANCE - distance) / warning_range;
        let level = 4.0 + normalized * 2.0;
        level as u8
    } else if distance < NOTICE_DISTANCE {
        // notice zone (levels 1-3)
        let notice_range = NOTICE_DISTANCE - WARNING_DISTANCE;
        let normalized = (NOTICE_DISTANCE - distance) / notice_range;
        let level = 1.0 + normalized * 2.0;
        level as u8
    } else {
        // beyond notice zone
        0
    }
}

// haptic feedback patterns for different intensities
async fn provide_haptic_feedback(motor: &mut Output<'_>, intensity: u8) {
    match intensity {
        10 => { // maximum intensity
            motor.set_high();
            Timer::after(Duration::from_millis(80)).await;
        },
        9 => { // very strong
            motor.set_high();
            Timer::after(Duration::from_millis(80)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(20)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(80)).await;
        },
        8 => { // strong
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
        5 => { // medium
            motor.set_high();
            Timer::after(Duration::from_millis(40)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(60)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(40)).await;
        },
        4 => { // light-medium
            motor.set_high();
            Timer::after(Duration::from_millis(30)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(70)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(30)).await;
        },
        3 => { // light
            motor.set_high();
            Timer::after(Duration::from_millis(20)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(80)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(20)).await;
        },
        2 => { // very light
            motor.set_high();
            Timer::after(Duration::from_millis(10)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(90)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(10)).await;
        },
        1 => { // minimal
            motor.set_high();
            Timer::after(Duration::from_millis(5)).await;
            motor.set_low();
            Timer::after(Duration::from_millis(95)).await;
            motor.set_high();
            Timer::after(Duration::from_millis(5)).await;
        },
        _ => { // no vibration
            motor.set_low();
            Timer::after(Duration::from_millis(10)).await;
        }
    }
}

// warning sounds for different distance ranges
async fn provide_warning_sound(buzzer: &mut Output<'_>, distance: f32) {
    if distance < 10.0 {
        // very close - rapid beeping
        for _ in 0..3 {
            buzzer.set_high();
            Timer::after(Duration::from_millis(25)).await;
            buzzer.set_low();
            Timer::after(Duration::from_millis(25)).await;
        }
    } else if distance < 20.0 {
        // medium close - moderate beeping
        for _ in 0..2 {
            buzzer.set_high();
            Timer::after(Duration::from_millis(50)).await;
            buzzer.set_low();
            Timer::after(Duration::from_millis(50)).await;
        }
    } else {
        // not as close - single beep
        buzzer.set_high();
        Timer::after(Duration::from_millis(70)).await;
        buzzer.set_low();
    }
}