use std::io::Cursor;
use std::thread;
use std::{sync::Arc, time::Duration};

use esp_idf_hal::delay::FreeRtos;
use esp_idf_sys::EspError;
use image::imageops::FilterType;
use image::io::Reader as ImageReader;
use image::{DynamicImage, GenericImageView};

use esp_idf_hal::{
    peripherals::Peripherals,
    prelude::*, gpio::{self, InputPin, OutputPin}, peripheral::Peripheral,
};
// delay::{Ets, FreeRtos},
// gpio::{*, self},
// i2c::*,

use esp_idf_sys::camera::{self, camera_config_t__bindgen_ty_1, camera_config_t__bindgen_ty_2};


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
const CAM_PIN_PWDN: ::std::os::raw::c_int = 26;   // define PWDN_GPIO_NUM -1
const CAM_PIN_RESET: ::std::os::raw::c_int = -1; //software reset will be performed   // define RESET_GPIO_NUM -1
const CAM_PIN_XCLK: ::std::os::raw::c_int = 4;   // define XCLK_GPIO_NUM 32
const CAM_PIN_SIOD: ::std::os::raw::c_int = 18;   // define SIOD_GPIO_NUM 13
const CAM_PIN_SIOC: ::std::os::raw::c_int = 23;   // define SIOC_GPIO_NUM 12
const CAM_PIN_D7: ::std::os::raw::c_int = 36;   // define Y9_GPIO_NUM 39
const CAM_PIN_D6: ::std::os::raw::c_int = 15;   // define Y8_GPIO_NUM 36
const CAM_PIN_D5: ::std::os::raw::c_int = 12;   // define Y7_GPIO_NUM 23
const CAM_PIN_D4: ::std::os::raw::c_int = 39;   // define Y6_GPIO_NUM 18
const CAM_PIN_D3: ::std::os::raw::c_int = 35;   // define Y5_GPIO_NUM 15
const CAM_PIN_D2: ::std::os::raw::c_int = 14;   // define Y4_GPIO_NUM 4
const CAM_PIN_D1: ::std::os::raw::c_int = 13;   // define Y3_GPIO_NUM 14
const CAM_PIN_D0: ::std::os::raw::c_int = 34;   // define Y2_GPIO_NUM 5
const CAM_PIN_VSYNC: ::std::os::raw::c_int = 5;   // define VSYNC_GPIO_NUM 27
const CAM_PIN_HREF: ::std::os::raw::c_int = 27;   // define HREF_GPIO_NUM 25
const CAM_PIN_PCLK: ::std::os::raw::c_int = 25;   // define PCLK_GPIO_NUM 19

fn init_camera() -> Result<(), EspError> {

    let config = camera::camera_config_t {
        pin_pwdn: CAM_PIN_PWDN,
        pin_reset: CAM_PIN_RESET,
        pin_xclk: CAM_PIN_XCLK,
        __bindgen_anon_1: camera_config_t__bindgen_ty_1{
            pin_sccb_sda: CAM_PIN_SIOD,
        },
        __bindgen_anon_2: camera_config_t__bindgen_ty_2{
            pin_sccb_scl: CAM_PIN_SIOC,
        },

        pin_d7: CAM_PIN_D7,
        pin_d6: CAM_PIN_D6,
        pin_d5: CAM_PIN_D5,
        pin_d4: CAM_PIN_D4,
        pin_d3: CAM_PIN_D3,
        pin_d2: CAM_PIN_D2,
        pin_d1: CAM_PIN_D1,
        pin_d0: CAM_PIN_D0,
        pin_vsync: CAM_PIN_VSYNC,
        pin_href: CAM_PIN_HREF,
        pin_pclk: CAM_PIN_PCLK,

        xclk_freq_hz: 20000000,
        ledc_timer: camera::ledc_timer_t_LEDC_TIMER_0,
        ledc_channel: camera::ledc_channel_t_LEDC_CHANNEL_0,
        pixel_format: camera::pixformat_t_PIXFORMAT_JPEG,
        frame_size: camera::framesize_t_FRAMESIZE_QVGA,
        jpeg_quality: 12,
        fb_count: 1,
        fb_location: camera::camera_fb_location_t_CAMERA_FB_IN_PSRAM,
        grab_mode: camera::camera_grab_mode_t_CAMERA_GRAB_WHEN_EMPTY,
        sccb_i2c_port: -1,
    };

    esp_idf_sys::esp!(unsafe { camera::esp_camera_init(&config) })
}

fn take_picture() {
    unsafe {
        let pic = camera::esp_camera_fb_get();
        let mut buf: *mut u8 = libc::malloc(std::mem::size_of::<u8>()) as *mut u8;
        let mut buf_len = 0;

        camera::frame2bmp(pic, &mut buf, &mut buf_len);

        let slice = std::slice::from_raw_parts_mut(buf, buf_len);

        // println!("Read");
        let img = ImageReader::new(Cursor::new(slice))
            .with_guessed_format()
            .unwrap();

        // println!("Decode");
        match img.decode() {
            Ok(img) => image_to_ascii(img),
            Err(err) => {
                dbg!(err);
            }
        };
        libc::free(buf as *mut libc::c_void);
        camera::esp_camera_fb_return(pic);
    }
}

fn image_to_ascii(image: DynamicImage) {
    let resolution = 8;
    let pallete: [char; 7] = [' ', '.', '/', '*', '#', '$', '@'];

    let mut y = 0;
    let small_img = image.resize_exact(
        image.width() / (resolution / 2),
        image.height() / resolution,
        FilterType::Nearest,
    );

    for p in small_img.pixels() {
        if y != p.1 {
            println!();
            y = p.1;
        }

        let r = p.2 .0[0] as f32;
        let g = p.2 .0[1] as f32;
        let b = p.2 .0[2] as f32;

        let k = r * 0.3 + g * 0.59 + b * 0.11;
        let character = ((k / 255.0) * (pallete.len() - 1) as f32).round() as usize;
        print!("{}", pallete[character]);
    }
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    // // OV2640 Camera	ESP32
    // // PWD	GPIO 26
    // let pin_pwdn = peripherals.pins.gpio26;
    // // RST	not connected (-1)
    // let pin_reset = -1;
    // // XCLK	GPIO 32
    // let pin_xclk = peripherals.pins.gpio32;
    // // Y2	GPIO 5
    // let pin_d0 = peripherals.pins.gpio5;
    // // Y3	GPIO 14
    // let pin_d1 = peripherals.pins.gpio14;
    // // Y4	GPIO 4
    // let pin_d2 = peripherals.pins.gpio4;
    // // Y5	GPIO 15
    // let pin_d3 = peripherals.pins.gpio15;
    // // Y6	GPIO 18
    // let pin_d4 = peripherals.pins.gpio18;
    // // Y7	GPIO 23
    // let pin_d5 = peripherals.pins.gpio23;
    // // Y8	GPIO 36
    // let pin_d6 = peripherals.pins.gpio36;
    // // Y9	GPIO 39
    // let pin_d7 = peripherals.pins.gpio39;
    // // VSYNC	GPIO 27
    // let pin_vsync = peripherals.pins.gpio27;
    // // HREF	GPIO 25
    // let pin_href = peripherals.pins.gpio25;
    // // PCLK	GPIO 19
    // let pin_pclk = peripherals.pins.gpio19;


    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Hello, world!");

    if init_camera().is_ok() {
        // Reset terminal
        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
        loop {
            // Move to the top left
            print!("{esc}[1;1H", esc = 27 as char);
            take_picture();
            FreeRtos::delay_ms(2000);
        }
    } else {
        loop {
            FreeRtos::delay_ms(2000);
            log::info!("Hello, world!");
        }

    }

}

