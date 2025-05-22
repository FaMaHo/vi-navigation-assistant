use embassy_net::{Stack, tcp::TcpSocket};
use defmt::*;
use embedded_io_async::{Read, Write};
use core::fmt::Write as FmtWrite;
use heapless::String;

// Shared state for the current sensor readings
pub static mut LEFT_DISTANCE: f32 = 100.0;
pub static mut RIGHT_DISTANCE: f32 = 100.0;

#[embassy_executor::task]
pub async fn tcp_server_task(_stack: &'static Stack<'static>, mut socket: TcpSocket<'static>) {
    info!("TCP server task started");
    
    loop {
        // Listen for connections on port 8080
        info!("TCP server listening on port 8080...");
        if let Err(e) = socket.accept(8080).await {
            warn!("Failed to accept connection: {:?}", e);
            continue;
        }
        
        info!("TCP connection accepted!");
        
        // Handle the connection
        handle_tcp_connection(&mut socket).await;
        
        // Close the connection
        socket.close();
        
        // Small delay before accepting next connection
        embassy_time::Timer::after_secs(1).await;
    }
}

async fn handle_tcp_connection(socket: &mut TcpSocket<'_>) {
    let mut rx_buffer = [0; 512];
    
    // Read request (we don't actually use it, but we need to read it)
    match socket.read(&mut rx_buffer).await {
        Ok(n) => {
            info!("Read {} bytes", n);
        }
        Err(e) => {
            warn!("Failed to read from socket: {:?}", e);
            return;
        }
    }
    
    // Get current distances
    let left = unsafe { LEFT_DISTANCE };
    let right = unsafe { RIGHT_DISTANCE };
    
    // Format response
    let mut response: String<64> = String::new();
    let _ = FmtWrite::write_str(&mut response, "L:");
    let _ = FmtWrite::write_fmt(&mut response, format_args!("{}", left as u32));
    let _ = FmtWrite::write_str(&mut response, " R:");
    let _ = FmtWrite::write_fmt(&mut response, format_args!("{}", right as u32));
    
    // Send response
    if let Err(e) = socket.write_all(response.as_bytes()).await {
        warn!("Failed to write to socket: {:?}", e);
    }
}