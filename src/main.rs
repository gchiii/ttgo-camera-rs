pub mod lib;

use esp_idf_hal::{
    delay::{Ets, FreeRtos},
    gpio::*,
    i2c::*,
    peripherals::Peripherals,
    prelude::*,
};


// PIR sensor on GPIO 33
// const PIR_DETECT

// SDA on GPIO 21
// SCL on GPIO 22

// Button on GPIO 34

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

// OV2640 Camera	ESP32
// Y9	GPIO 39
// Y8	GPIO 36
// Y7	GPIO 23
// Y6	GPIO 18
// Y5	GPIO 15
// Y4	GPIO 4
// Y3	GPIO 14
// Y2	GPIO 5
// VSYNC	GPIO 27
// HREF	GPIO 25
// PCLK	GPIO 19
// PWD	GPIO 26
// XCLK	GPIO 32
// SIOD	GPIO 13
// SIOC	GPIO 12
// RST	not connected (-1)

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // let camera_config = esp_idf_sys::camera_config_t{
    //     pin_pwdn : -1,
    //     pin_reset : -1,
    //     pin_xclk : 32,
    //     pin_sscb_sda : 13,
    //     pin_sscb_scl : 12,

    //     pin_d7 : 39,
    //     pin_d6 : 36,
    //     pin_d5 : 23,
    //     pin_d4 : 18,
    //     pin_d3 : 15,
    //     pin_d2 : 4,
    //     pin_d1 : 14,
    //     pin_d0 : 5,
    //     pin_vsync : 27,
    //     pin_href : 25,
    //     pin_pclk : 19,

    //     //XCLK 20MHz or 10MHz for OV2640 double FPS (Experimental)
    //     xclk_freq_hz : 20000000,
    //     ledc_timer : esp_idf_sys::ledc_timer_t_LEDC_TIMER_0,
    //     ledc_channel : esp_idf_sys::ledc_channel_t_LEDC_CHANNEL_0,

    //     pixel_format : esp_idf_sys::pixformat_t_PIXFORMAT_JPEG, //YUV422,GRAYSCALE,RGB565,JPEG
    //     frame_size : esp_idf_sys::framesize_t_FRAMESIZE_QVGA ,    //QQVGA-UXGA Do not use sizes above QVGA when not JPEG

    //     jpeg_quality : 12, //0-63 lower number means higher quality
    //     fb_count : 1,       //if more than one, i2s runs in continuous mode. Use only with JPEG
    //     fb_location: esp_idf_sys::camera_fb_location_t_CAMERA_FB_IN_PSRAM,
    //     grab_mode: esp_idf_sys::camera_grab_mode_t_CAMERA_GRAB_WHEN_EMPTY
    // };

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");
}
