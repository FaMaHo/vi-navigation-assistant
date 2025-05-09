#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_time::{Duration, Timer, Instant};
use {defmt_rtt as _, panic_probe as _};
use defmt::*;

// Import interrupts definition module
mod irqs;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Get a handle to the RP's peripherals
    let p = embassy_rp::init(Default::default());
    
    // Configure GPIO pins for the ultrasonic sensor
    let trigger = Output::new(p.PIN_14, Level::Low);  // Trigger pin
    let echo = Input::new(p.PIN_15, Pull::None);      // Echo pin
    
    // Configure GPIO pin for the buzzer
    let mut buzzer = Output::new(p.PIN_16, Level::Low);   // Buzzer pin
    
    // Create the ultrasonic sensor handler
    let mut ultrasonic = UltrasonicSensor {
        trigger,
        echo,
    };
    
    info!("Starting ultrasonic sensor with buzzer feedback!");
    
    // Main loop
    loop {
        // Measure distance
        match ultrasonic.measure_distance().await {
            Ok(distance_cm) => {
                info!("Distance: {} cm", distance_cm);
                
                // Create more distinctive feedback based on distance zones
                if distance_cm < 15.0 {
                    // Very close (0-15cm): Continuous tone
                    info!("VERY CLOSE!");
                    buzzer.set_high();
                    Timer::after(Duration::from_millis(500)).await;
                } else if distance_cm < 30.0 {
                    // Close (15-30cm): Rapid beeps (3 quick beeps)
                    info!("CLOSE!");
                    for _ in 0..3 {
                        buzzer.set_high();
                        Timer::after(Duration::from_millis(50)).await;
                        buzzer.set_low();
                        Timer::after(Duration::from_millis(50)).await;
                    }
                    Timer::after(Duration::from_millis(300)).await;
                } else if distance_cm < 60.0 {
                    // Medium distance (30-60cm): Double beep
                    info!("MEDIUM distance");
                    for _ in 0..2 {
                        buzzer.set_high();
                        Timer::after(Duration::from_millis(100)).await;
                        buzzer.set_low();
                        Timer::after(Duration::from_millis(100)).await;
                    }
                    Timer::after(Duration::from_millis(400)).await;
                } else if distance_cm < 100.0 {
                    // Far (60-100cm): Single beep
                    info!("FAR");
                    buzzer.set_high();
                    Timer::after(Duration::from_millis(100)).await;
                    buzzer.set_low();
                    Timer::after(Duration::from_millis(700)).await;
                } else {
                    // Out of range (>100cm): No beep
                    buzzer.set_low();
                    Timer::after(Duration::from_millis(500)).await;
                }
            },
            Err(e) => {
                info!("Error measuring distance: {}", e);
                Timer::after(Duration::from_millis(500)).await;
            }
        }
    }
}

struct UltrasonicSensor<'d> {
    trigger: Output<'d>,
    echo: Input<'d>,
}

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
        
        Ok(distance_cm)
    }
}