// CYW43 WiFi firmware
pub const FIRMWARE: &[u8] = include_bytes!("../firmware/43439A0.bin");

// WiFi configuration
pub const WIFI_NETWORK: &str = "VisionAssist";
pub const WIFI_PASSWORD: &str = ""; // Empty for open networks