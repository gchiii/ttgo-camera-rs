// pub mod lib;

// use esp_idf_sys::camera::camera_config_t;

use esp_idf_hal::{
    peripherals::Peripherals,
    prelude::*, gpio,
};
// delay::{Ets, FreeRtos},
// gpio::{*, self},
// i2c::*,

use ttgo_camera::Camera;


// Chip: ESP32-WROVER-B
// Protocol: Wi-Fi 802.11 b/g/n & bluetooth 4.2 BLE & BR/EDR
// Flash: 4MB
// PSRAM: 8MB
// Display chip: SSD1306 I2C
// Display type: OLED
// Display resolution: 128×64
// PIR: AS312
// UART chip: CP2104
// Charging chip: IP5306 I2C
// Camera: OV2640
// Camera Resolution: 2 Megapixel

// this is from https://makeradvisor.com/esp32-ttgo-t-camera-pir-sensor-oled/
// define PWDN_GPIO_NUM -1
// define RESET_GPIO_NUM -1
// define XCLK_GPIO_NUM 32
// define SIOD_GPIO_NUM 13
// define SIOC_GPIO_NUM 12
// define Y9_GPIO_NUM 39
// define Y8_GPIO_NUM 36
// define Y7_GPIO_NUM 23
// define Y6_GPIO_NUM 18
// define Y5_GPIO_NUM 15
// define Y4_GPIO_NUM 4
// define Y3_GPIO_NUM 14
// define Y2_GPIO_NUM 5
// define VSYNC_GPIO_NUM 27
// define HREF_GPIO_NUM 25
// define PCLK_GPIO_NUM 19
fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    // OV2640 Camera	ESP32
    // PWD	GPIO 26
    let pin_pwdn = peripherals.pins.gpio26;
    // RST	not connected (-1)
    let pin_reset = -1;
    // XCLK	GPIO 32
    let pin_xclk = peripherals.pins.gpio32;
    // Y2	GPIO 5
    let pin_d0 = peripherals.pins.gpio5;
    // Y3	GPIO 14
    let pin_d1 = peripherals.pins.gpio14;
    // Y4	GPIO 4
    let pin_d2 = peripherals.pins.gpio4;
    // Y5	GPIO 15
    let pin_d3 = peripherals.pins.gpio15;
    // Y6	GPIO 18
    let pin_d4 = peripherals.pins.gpio18;
    // Y7	GPIO 23
    let pin_d5 = peripherals.pins.gpio23;
    // Y8	GPIO 36
    let pin_d6 = peripherals.pins.gpio36;
    // Y9	GPIO 39
    let pin_d7 = peripherals.pins.gpio39;
    // VSYNC	GPIO 27
    let pin_vsync = peripherals.pins.gpio27;
    // HREF	GPIO 25
    let pin_href = peripherals.pins.gpio25;
    // PCLK	GPIO 19
    let pin_pclk = peripherals.pins.gpio19;
    // SIOD	GPIO 13

    // SIOC	GPIO 12


    let cam = Camera::new(
        pin_pwdn, pin_reset.into(), pin_xclk,
        pin_d0, pin_d1, pin_d2, pin_d3, pin_d4, pin_d5, pin_d6, pin_d7,
        pin_vsync, pin_href, pin_pclk).unwrap();
































    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");
}
