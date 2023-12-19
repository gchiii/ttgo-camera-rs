use std::io::Cursor;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_sys::EspError;
use image::imageops::FilterType;
use image::io::Reader as ImageReader;
use image::{DynamicImage, GenericImageView, ImageError};

use esp_idf_hal::{
    peripherals::Peripherals,
    prelude::*,
};

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



fn alt_take_pic() -> Result<DynamicImage, ImageError>{
    let fb = unsafe { camera::esp_camera_fb_get() };

    let mut buf: *mut u8 = unsafe { libc::malloc(std::mem::size_of::<u8>()) as *mut u8 };
    let mut buf_len = 0;

    unsafe { camera::frame2bmp(fb, &mut buf, &mut buf_len) } ;

    let photo = unsafe { std::slice::from_raw_parts_mut(buf, buf_len) };

    // let photo = unsafe { std::slice::from_raw_parts((*fb).buf, (*fb).len) };

    println!("Read");
    let img = ImageReader::new(Cursor::new(photo))
        .with_guessed_format()?
        .decode()?;
    unsafe {
        libc::free(buf as *mut libc::c_void);
        camera::esp_camera_fb_return(fb);
    }

    Ok(img)
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

use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    mono_font::{ascii::FONT_6X12, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
    primitives::{Circle, PrimitiveStyleBuilder, Rectangle, Triangle},
};
use ttgo_camera::Camera;


fn draw_shapes<D: DrawTarget<Color = BinaryColor>>(display: &mut D) where <D as DrawTarget>::Error: std::fmt::Debug
{
    let yoffset = 8;

    let style = PrimitiveStyleBuilder::new()
        .stroke_width(1)
        .stroke_color(BinaryColor::On)
        .build();

    // screen outline
    // default display size is 128x64 if you don't pass a _DisplaySize_
    // enum to the _Builder_ struct
    Rectangle::new(Point::new(0, 0), Size::new(127, 31))
        .into_styled(style)
        .draw(display)
        .unwrap();

    // triangle
    Triangle::new(
        Point::new(16, 16 + yoffset),
        Point::new(16 + 16, 16 + yoffset),
        Point::new(16 + 8, yoffset),
    )
    .into_styled(style)
    .draw(display)
    .unwrap();

    // square
    Rectangle::new(Point::new(52, yoffset), Size::new_equal(16))
        .into_styled(style)
        .draw(display)
        .unwrap();

    // circle
    Circle::new(Point::new(88, yoffset), 16)
        .into_styled(style)
        .draw(display)
        .unwrap();

}

fn draw_some_text<D: DrawTarget<Color = BinaryColor>>(display: &mut D) where <D as DrawTarget>::Error: std::fmt::Debug
{
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X12)
        .text_color(BinaryColor::On)
        .build();

    Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
        .draw(display)
        .unwrap();

    Text::with_baseline("Hello Rust!", Point::new(0, 16), text_style, Baseline::Top)
        .draw(display)
        .unwrap();
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    let mut peripherals = Peripherals::take().unwrap();
    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;

    let config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config).unwrap();
    let interface = I2CDisplayInterface::new(i2c);
    // Command::AllOn(true).send(&mut interface).unwrap();
    // FreeRtos::delay_ms(2000);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate180)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    draw_shapes(&mut display);
    display.flush().unwrap();
    FreeRtos::delay_ms(2000);

    // draw_some_text(&mut display);
    // display.flush().unwrap();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Hello, world!");

    let cam_sda = (&mut peripherals.pins.gpio18).into_ref().map_into();
    let cam_scl = (&mut peripherals.pins.gpio23).into_ref().map_into();
    let cam_pwdn = (&mut peripherals.pins.gpio26).into_ref().map_into();
    let p_camera = Camera::new(
        Some(cam_pwdn),
        None,
        &mut peripherals.pins.gpio4,
        &mut peripherals.pins.gpio34,
        &mut peripherals.pins.gpio13,
        &mut peripherals.pins.gpio14,
        &mut peripherals.pins.gpio35,
        &mut peripherals.pins.gpio39,
        &mut peripherals.pins.gpio12,
        &mut peripherals.pins.gpio15,
        &mut peripherals.pins.gpio36,
        &mut peripherals.pins.gpio5,
        &mut peripherals.pins.gpio27,
        &mut peripherals.pins.gpio25,
        Some(cam_sda),
        Some(cam_scl),
    );
    let camera = match p_camera {
        Ok(c) => c,
        Err(e) => {
            log::error!("{}", e);
            return;
        },
    };
    let _sensor = camera.sensor();
    // if let Err(e) = _sensor.set_pixformat(camera::pixformat_t_PIXFORMAT_JPEG) {
    //     log::error!("{}", e);
    // }
    if let Err(e) = _sensor.init_status() {
        log::error!("{}", e);
    }
    // if let Err(e) = _sensor. {
    //     log::error!("{}", e);
    // }
    loop {
        println!("Read");
        if let Ok(pic) = alt_take_pic() {
            // Move to the top left
            print!("{esc}[1;1H", esc = 27 as char);
            // pic.
            image_to_ascii(pic);
        }
        // if let Some(fb) = camera.get_framebuffer() {
        //     match fb.data_as_jpeg(20) {
        //         Ok(pic) => {
        //             if let Ok(img_reader) = ImageReader::new(Cursor::new(pic)).with_guessed_format() {
        //                 match img_reader.decode() {
        //                     Ok(image) => image_to_ascii(image),
        //                     Err(e) => log::error!("{}", e),
        //                 }
        //             }
        //         }
        //         Err(e)=> log::error!("{}", e),
        //     }
        // }
        FreeRtos::delay_ms(2000);
    }

    // if init_camera().is_ok() {
    //     // Reset terminal
    //     print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
    //     while let Ok(pic) = alt_take_pic() {
    //         // Move to the top left
    //         print!("{esc}[1;1H", esc = 27 as char);
    //         // pic.
    //         image_to_ascii(pic);
    //         FreeRtos::delay_ms(2000);
    //     }
    // } else {
    //     loop {
    //         FreeRtos::delay_ms(2000);
    //         log::info!("Hello, world!");
    //     }
    // }

}

