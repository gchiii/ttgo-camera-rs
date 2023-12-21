use std::io::{Cursor, Write, Read};
use std::net::TcpStream;
use std::{thread::sleep, time::Duration};

use esp_idf_svc::hal::{
    delay::FreeRtos,
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    peripheral::Peripheral,
    prelude::*
};
// use embedded_svc::utils::io;
// use embedded_svc::{
//     http::client::Client,
//     io::Write,
//     wifi::{ClientConfiguration, Configuration},
// };
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::EspWifi};
use esp_idf_svc::hal::peripheral;
use esp_idf_svc::{wifi::*, ipv4};
use esp_idf_svc::eventloop::*;
use esp_idf_svc::timer::*;
use log::*;
use esp_idf_svc::ping;

use image::imageops::FilterType;
use image::io::Reader as ImageReader;
use image::{DynamicImage, GenericImageView};

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

use anyhow::{anyhow, Error, bail, Result};

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
}

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
// const CAM_PIN_PWDN: ::std::os::raw::c_int = 26;   // define PWDN_GPIO_NUM -1
// const CAM_PIN_RESET: ::std::os::raw::c_int = -1; //software reset will be performed   // define RESET_GPIO_NUM -1
// const CAM_PIN_XCLK: ::std::os::raw::c_int = 4;   // define XCLK_GPIO_NUM 32
// const CAM_PIN_SIOD: ::std::os::raw::c_int = 18;   // define SIOD_GPIO_NUM 13
// const CAM_PIN_SIOC: ::std::os::raw::c_int = 23;   // define SIOC_GPIO_NUM 12
// const CAM_PIN_D7: ::std::os::raw::c_int = 36;   // define Y9_GPIO_NUM 39
// const CAM_PIN_D6: ::std::os::raw::c_int = 15;   // define Y8_GPIO_NUM 36
// const CAM_PIN_D5: ::std::os::raw::c_int = 12;   // define Y7_GPIO_NUM 23
// const CAM_PIN_D4: ::std::os::raw::c_int = 39;   // define Y6_GPIO_NUM 18
// const CAM_PIN_D3: ::std::os::raw::c_int = 35;   // define Y5_GPIO_NUM 15
// const CAM_PIN_D2: ::std::os::raw::c_int = 14;   // define Y4_GPIO_NUM 4
// const CAM_PIN_D1: ::std::os::raw::c_int = 13;   // define Y3_GPIO_NUM 14
// const CAM_PIN_D0: ::std::os::raw::c_int = 34;   // define Y2_GPIO_NUM 5
// const CAM_PIN_VSYNC: ::std::os::raw::c_int = 5;   // define VSYNC_GPIO_NUM 27
// const CAM_PIN_HREF: ::std::os::raw::c_int = 27;   // define HREF_GPIO_NUM 25
// const CAM_PIN_PCLK: ::std::os::raw::c_int = 25;   // define PCLK_GPIO_NUM 19



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

fn take_pic(camera: &Camera<'_>) -> Result<DynamicImage, Error> {
    if let Some(fb) = camera.get_framebuffer() {
        let pic = fb.data_as_bmp()?;
        let image = ImageReader::new(Cursor::new(pic))
            .with_guessed_format()?
            .decode()?;
        Ok(image)
    } else {
        Err(anyhow!("Failed to get framebuffer"))
    }
}

fn ping(ip: ipv4::Ipv4Addr) -> Result<()> {
    info!("About to do some pings for {:?}", ip);

    let ping_summary = ping::EspPing::default().ping(ip, &Default::default())?;
    if ping_summary.transmitted != ping_summary.received {
        bail!("Pinging IP {} resulted in timeouts", ip);
    }

    info!("Pinging done");

    Ok(())
}

fn wifi(
    modem: impl peripheral::Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
    nvs: Option<EspDefaultNvsPartition>,
) -> Result<Box<EspWifi<'static>>> {
    let app_config = CONFIG;
    let ssid: &str = app_config.wifi_ssid;
    let pass = app_config.wifi_psk;

    info!("ssid = {} pw = {}", ssid, pass);

    let mut esp_wifi = EspWifi::new(modem, sysloop.clone(), nvs)?;

    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;

    info!("Starting wifi...");

    wifi.start()?;

    info!("Scanning...");

    let mut ap_infos = wifi.scan()?;

    for ap in &ap_infos {
        info!("ap {:?}", ap);
    }
    let mut ours = ap_infos.into_iter().find(|a| a.ssid == ssid);
    if ours.is_none() {
        ap_infos = wifi.scan()?;
        for ap in &ap_infos {
            info!("ap {:?}", ap);
        }
        ours = ap_infos.into_iter().find(|a| a.ssid == ssid);
    }

    let channel = if let Some(ours) = ours {
        info!(
            "Found configured access point {} on channel {}",
            ssid, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            ssid
        );
        None
    };
    // let client_config = ClientConfiguration {
    //     ssid: ssid.into(),
    //     password: pass.into(),
    //     channel,
    //     ..Default::default()
    // };
    // wifi.set_configuration(&Configuration::Client(client_config))?;
    wifi.set_configuration(&Configuration::Mixed(
        ClientConfiguration {
            ssid: ssid.into(),
            password: pass.into(),
            channel,
            ..Default::default()
        },
        AccessPointConfiguration {
            ssid: "aptest".into(),
            channel: channel.unwrap_or(1),
            ..Default::default()
        },
    ))?;

    info!("Connecting wifi...");

    wifi.connect()?;

    info!("Waiting for DHCP lease...");

    if let Err(e) = wifi.wait_netif_up() {
        error!("wifi error: {:?}", e);
        wifi.connect()?;
    }
    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    info!("Wifi DHCP info: {:?}", ip_info);

    ping(ip_info.subnet.gateway)?;

    Ok(Box::new(esp_wifi))
}

fn test_tcp() -> Result<()> {
    info!("About to open a TCP connection to 1.1.1.1 port 80");

    let mut stream = TcpStream::connect("one.one.one.one:80")?;

    let err = stream.try_clone();
    if let Err(err) = err {
        info!(
            "Duplication of file descriptors does not work (yet) on the ESP-IDF, as expected: {}",
            err
        );
    }

    stream.write_all("GET / HTTP/1.0\n\n".as_bytes())?;

    let mut result = Vec::new();

    stream.read_to_end(&mut result)?;

    info!(
        "1.1.1.1 returned:\n=================\n{}\n=================\nSince it returned something, all is OK",
        std::str::from_utf8(&result)?);

    Ok(())
}

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Hello, world!");

    let mut peripherals = Peripherals::take()?;

    let sys_loop =  EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    // let mut wifi_driver = match EspWifi::new(peripherals.modem, sys_loop, Some(nvs)) {
    //     Ok(w) => w,
    //     Err(e) => {
    //         log::error!("error {:?}", e);
    //         return;
    //     },
    // };

    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;

    let config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c = I2cDriver::new(i2c, sda, scl, &config)?;
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate180)
        .into_buffered_graphics_mode();

    display.init().map_err(|err| anyhow!("{:?}", err))?;

    draw_shapes(&mut display);
    display.flush().map_err(|err| anyhow!("{:?}", err))?;

    let mut wifi = wifi(peripherals.modem, sys_loop.clone(), Some(nvs))?;

    FreeRtos::delay_ms(2000);
    test_tcp()?;

    // draw_some_text(&mut display);
    // display.flush().unwrap();

    let cam_sda = (&mut peripherals.pins.gpio18).into_ref().map_into();
    let cam_scl = (&mut peripherals.pins.gpio23).into_ref().map_into();
    let cam_pwdn = (&mut peripherals.pins.gpio26).into_ref().map_into();
    let camera = Camera::new(
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
    )?;
    // let camera: Camera<'_> = match p_camera {
    //     Ok(c) => c,
    //     Err(e) => {
    //         log::error!("{}", e);
    //         return;
    //     },
    // };
    let _sensor = camera.sensor();
    // if let Err(e) = _sensor.set_pixformat(camera::pixformat_t_PIXFORMAT_GRAYSCALE) {
    //     log::error!("{}", e);
    // }
    if let Err(e) = _sensor.init_status() {
        log::error!("{}", e);
    }
    // if let Err(e) = _sensor.set_framesize(framesize) {
    //     log::error!("{}", e);
    // }
    // match wifi_driver.sta_netif().get_ip_info() {
    //     Ok(ipinfo) => log::info!("IP info: {:?}", ipinfo),
    //     Err(e) => log::error!("error {:?}", e),
    // }

    loop {
        match take_pic(&camera) {
            Ok(image) => {
                // Move to the top left
                // print!("{esc}[1;1H", esc = 27 as char);
                // pic.
                image_to_ascii(image);
            },
            Err(e) => log::error!("{}", e),
        }
        FreeRtos::delay_ms(2000);
        // match wifi_driver.sta_netif().get_ip_info() {
        //     Ok(ipinfo) => log::info!("IP info: {:?}", ipinfo),
        //     Err(e) => log::error!("error {:?}", e),
        // }
    }
}

