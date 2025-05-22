use embassy_executor::Spawner;
use embassy_net::Config;
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    peripherals::{DMA_CH2, PIN_23, PIN_24, PIN_25, PIN_29, PIO0},
    pio::{Pio, InterruptHandler as PioInterruptHandler},
};
use static_cell::StaticCell;
use cyw43_pio::PioSpi;
use embassy_lab_utils::init_network_stack as lab_init_network_stack;
use fixed::types::U24F8;
use defmt::{info, warn};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
});

// CYW43 WiFi firmware
pub const FIRMWARE: &[u8] = include_bytes!("../cyw43-firmware/43439A0.bin");
pub const CLM: &[u8] = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

// WiFi AP configuration
pub const AP_SSID: &str = "VisionAssist";
pub const AP_CHANNEL: u8 = 6; // WiFi channel (1-11)

static STATE: StaticCell<cyw43::State> = StaticCell::new();

// Simple init function that returns what we need
pub async fn init_wifi(
    spawner: &Spawner,
    pin_23: PIN_23,
    pin_24: PIN_24,
    pin_25: PIN_25, 
    pin_29: PIN_29,
    pio0: PIO0,
    dma: DMA_CH2,
) -> (cyw43::NetDriver<'static>, cyw43::Control<'static>) {
    let pwr = Output::new(pin_23, Level::Low);
    let cs = Output::new(pin_25, Level::High);
    let mut pio = Pio::new(pio0, Irqs);
    
    // Create PioSpi with correct parameters and clock divider
    let spi = PioSpi::new(
        &mut pio.common, 
        pio.sm0, 
        U24F8::from_num(125_000_000.0 / 1_000_000.0), // 125MHz / 1MHz = 125
        pio.irq0, 
        cs, 
        pin_24, 
        pin_29, 
        dma
    );

    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, FIRMWARE).await;
    spawner.spawn(wifi_task(runner)).unwrap();

    // Initialize with CLM firmware
    control.init(CLM).await;
    control.set_power_management(cyw43::PowerManagementMode::PowerSave).await;

    (net_device, control)
}

pub async fn start_ap(control: &mut cyw43::Control<'static>) -> Result<(), &'static str> {
    info!("Starting WiFi Access Point '{}'...", AP_SSID);
    
    // Start AP mode using the correct API (SSID, channel)
    control.start_ap_open(AP_SSID, AP_CHANNEL).await;
    info!("WiFi Access Point '{}' started successfully on channel {}!", AP_SSID, AP_CHANNEL);
    Ok(())
}

pub async fn init_network_stack(
    spawner: &Spawner,
    pin_23: PIN_23,
    pin_24: PIN_24,
    pin_25: PIN_25,
    pin_29: PIN_29,
    pio0: PIO0,
    dma: DMA_CH2,
) -> (&'static embassy_net::Stack<'static>, embassy_net::tcp::TcpSocket<'static>) {
    // Initialize WiFi
    let (net_device, mut control) = init_wifi(spawner, pin_23, pin_24, pin_25, pin_29, pio0, dma).await;
    
    // Start AP mode
    match start_ap(&mut control).await {
        Ok(_) => info!("Access Point started successfully"),
        Err(e) => warn!("Failed to start Access Point: {}", e),
    }
    
    // Configure network stack with static IP for AP mode
    let config = Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(192, 168, 4, 1), 24),
        gateway: None,
        dns_servers: heapless::Vec::new(),
    });

    // Use the lab utils to initialize the network stack
    static STACK_RESOURCES: StaticCell<embassy_net::StackResources<4>> = StaticCell::new();
    static STACK: StaticCell<embassy_net::Stack<'static>> = StaticCell::new();
    
    let stack_instance = lab_init_network_stack(spawner, net_device, &STACK_RESOURCES, config);
    let stack = STACK.init(stack_instance);

    // Create TCP socket with buffers
    static RX_BUFFER: StaticCell<[u8; 1024]> = StaticCell::new();
    static TX_BUFFER: StaticCell<[u8; 1024]> = StaticCell::new();
    let rx_buffer = RX_BUFFER.init([0; 1024]);
    let tx_buffer = TX_BUFFER.init([0; 1024]);
    let socket = embassy_net::tcp::TcpSocket::new(*stack, rx_buffer, tx_buffer);

    info!("Network stack initialized with IP: 192.168.4.1");
    info!("Connect to WiFi network '{}' and browse to http://192.168.4.1", AP_SSID);
    info!("TCP server available on 192.168.4.1:8080");

    (stack, socket)
}

#[embassy_executor::task]
async fn wifi_task(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH2>>) -> ! {
    runner.run().await
}