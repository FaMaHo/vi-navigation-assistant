use embassy_net::{Stack, tcp::TcpSocket};
use defmt::*;
use embedded_io_async::{Read, Write};
use core::fmt::Write as FmtWrite;
use heapless::String;

#[embassy_executor::task]
pub async fn web_server_task(stack: &'static Stack<'static>) {
    info!("Web server task started");
    
    loop {
        // Create a new socket for each connection with fresh buffers
        let mut rx_buffer = [0; 1024];
        let mut tx_buffer = [0; 4096];
        let mut socket = TcpSocket::new(*stack, &mut rx_buffer, &mut tx_buffer);
        
        // Listen for connections on port 80
        info!("Web server listening on port 80...");
        if let Err(e) = socket.accept(80).await {
            warn!("Failed to accept connection: {:?}", e);
            continue;
        }
        
        info!("Web connection accepted!");
        
        // Handle the connection
        handle_web_connection(&mut socket).await;
        
        // Close the connection
        socket.close();
        
        // Small delay before accepting next connection
        embassy_time::Timer::after_millis(100).await;
    }
}

async fn handle_web_connection(socket: &mut TcpSocket<'_>) {
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
    
    // Generate HTTP response
    let response = generate_http_response();
    
    // Send response
    if let Err(e) = socket.write_all(response.as_bytes()).await {
        warn!("Failed to write to socket: {:?}", e);
    }
}

fn generate_http_response() -> String<2048> {
    let mut response = String::new();
    
    // Get current distances
    let left = unsafe { crate::tcp_server::LEFT_DISTANCE };
    let right = unsafe { crate::tcp_server::RIGHT_DISTANCE };
    
    // HTTP headers
    let _ = FmtWrite::write_str(&mut response, "HTTP/1.1 200 OK\r\n");
    let _ = FmtWrite::write_str(&mut response, "Content-Type: text/html\r\n");
    let _ = FmtWrite::write_str(&mut response, "Connection: close\r\n");
    let _ = FmtWrite::write_str(&mut response, "\r\n");
    
    // HTML content
    let _ = FmtWrite::write_str(&mut response, "<!DOCTYPE html>\n");
    let _ = FmtWrite::write_str(&mut response, "<html>\n");
    let _ = FmtWrite::write_str(&mut response, "<head>\n");
    let _ = FmtWrite::write_str(&mut response, "    <title>VisionAssist Status</title>\n");
    let _ = FmtWrite::write_str(&mut response, "    <meta http-equiv=\"refresh\" content=\"2\">\n");
    let _ = FmtWrite::write_str(&mut response, "    <style>\n");
    let _ = FmtWrite::write_str(&mut response, "        body { font-family: Arial, sans-serif; margin: 20px; }\n");
    let _ = FmtWrite::write_str(&mut response, "        .sensor { margin: 10px 0; padding: 10px; border: 1px solid #ccc; }\n");
    let _ = FmtWrite::write_str(&mut response, "        .critical { background-color: #ffcccc; }\n");
    let _ = FmtWrite::write_str(&mut response, "        .warning { background-color: #ffffcc; }\n");
    let _ = FmtWrite::write_str(&mut response, "        .normal { background-color: #ccffcc; }\n");
    let _ = FmtWrite::write_str(&mut response, "    </style>\n");
    let _ = FmtWrite::write_str(&mut response, "</head>\n");
    let _ = FmtWrite::write_str(&mut response, "<body>\n");
    let _ = FmtWrite::write_str(&mut response, "    <h1>VisionAssist Status</h1>\n");
    
    // Left sensor
    let _ = FmtWrite::write_str(&mut response, "    <div class=\"sensor ");
    if left < 30.0 {
        let _ = FmtWrite::write_str(&mut response, "critical");
    } else if left < 60.0 {
        let _ = FmtWrite::write_str(&mut response, "warning");
    } else {
        let _ = FmtWrite::write_str(&mut response, "normal");
    }
    let _ = FmtWrite::write_str(&mut response, "\">\n");
    let _ = FmtWrite::write_str(&mut response, "        <h2>Left Sensor</h2>\n");
    let _ = FmtWrite::write_fmt(&mut response, format_args!("        <p>Distance: {} cm</p>\n", left as u32));
    let _ = FmtWrite::write_str(&mut response, "    </div>\n");
    
    // Right sensor
    let _ = FmtWrite::write_str(&mut response, "    <div class=\"sensor ");
    if right < 30.0 {
        let _ = FmtWrite::write_str(&mut response, "critical");
    } else if right < 60.0 {
        let _ = FmtWrite::write_str(&mut response, "warning");
    } else {
        let _ = FmtWrite::write_str(&mut response, "normal");
    }
    let _ = FmtWrite::write_str(&mut response, "\">\n");
    let _ = FmtWrite::write_str(&mut response, "        <h2>Right Sensor</h2>\n");
    let _ = FmtWrite::write_fmt(&mut response, format_args!("        <p>Distance: {} cm</p>\n", right as u32));
    let _ = FmtWrite::write_str(&mut response, "    </div>\n");
    
    let _ = FmtWrite::write_str(&mut response, "</body>\n");
    let _ = FmtWrite::write_str(&mut response, "</html>\n");
    
    response
}