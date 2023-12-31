
use anyhow::{bail, Result};
use edge_executor::LocalExecutor;
use ssd1306::{rotation::DisplayRotation, size::DisplaySize128x64, mode::DisplayConfig};


use std::{
    time::{Instant, Duration},
    sync::{Arc, Mutex},
};

use esp_idf_hal::{reset::{ResetReason, WakeupReason}, gpio::PinDriver};
use esp_idf_svc::{
    hal::{
        peripherals::Peripherals,
        peripheral::Peripheral,
        timer::{Timer, TimerDriver}
    },
    io::Write,
    eventloop::EspSystemEventLoop,
    wifi::EspWifi,
    http::server::{Configuration, EspHttpServer},
};
use log::*;

// use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use ttgo_camera::{small_display::{init_display, draw_shapes}, wifi::init_wifi};
use ttgo_camera::esp_camera::Camera;
use ttgo_camera::wifi;


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
// PIR: AS312   - on GPIO 33
// UART chip: CP2104
// Charging chip: IP5306 I2C
// Camera: OV2640
// Camera Resolution: 2 Megapixel

// PIR input GPIO33
// BUTTON input GPIO 34

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




fn init_http(cam: Arc<Mutex<Camera>>) -> Result<EspHttpServer> {
    let mut httpd_config = Configuration::default();
    httpd_config.session_timeout = Duration::from_secs(5*50);
    httpd_config.uri_match_wildcard = true;
    let mut server = EspHttpServer::new(&httpd_config)?;

    server.fn_handler("/", esp_idf_svc::http::Method::Get, move |request| {
        let mut time = Instant::now();
        info!("handling request");
        let lock = cam.lock().unwrap(); // If a thread gets poisoned we're just fucked anyways
        let fb = match lock.get_framebuffer() {
            Some(fb) => fb,
            None => {
                let mut response = request.into_status_response(500)?;
                let _ = writeln!(response, "Error: Unable to get framebuffer");
                return Ok(());
            }
        };
        info!("got the framebuffer");
        // let jpeg = match fb.data_as_jpeg(20) {
        //     Ok(jpeg) => jpeg,
        //     Err(e) => {
        //         let mut response = request.into_status_response(500)?;
        //         let _ = writeln!(response, "init_http: Error: {:#?}", e);
        //         return Ok(());
        //     }
        // };
        let jpeg = fb.data();
        info!("Took {}ms to capture_jpeg", time.elapsed().as_millis());

        // Send the image
        time = Instant::now();
        let mut response = request.into_response(
            200,
            None,
            &[
                ("Content-Type", "image/jpeg"),
                ("Content-Length", &jpeg.len().to_string()),
            ],
        )?;

        let _ = response.write_all(jpeg);
        info!("Took {}ms to send image", time.elapsed().as_millis());

        Ok(())
    })?;

    Ok(server)
}


fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Hello, world!");

    let reset_reason = ResetReason::get();
    info!("Last reset was due to {:#?}", reset_reason);
    let wakeup_reason = WakeupReason::get();
    info!("Last wakeup was due to {:#?}", wakeup_reason);

    let executor: LocalExecutor = Default::default();
    edge_executor::block_on(executor.run(async_main()))

}

async fn async_main() -> Result<()> {
    info!("starting async_main");
    let mut peripherals = Peripherals::take()?;

    let sysloop =  EspSystemEventLoop::take()?;
    // let _nvs = EspDefaultNvsPartition::take()?;

    let i2c = peripherals.i2c0;
    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;
    let mut display = init_display(i2c, sda, scl, DisplaySize128x64, DisplayRotation::Rotate0)?
        .into_buffered_graphics_mode();
    display.init().map_err(|err| anyhow::anyhow!("{:?}", err))?;
    draw_shapes(&mut display);
    display.flush().map_err(|err| anyhow::anyhow!("{:?}", err))?;

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
    let camera_mutex = Arc::new(Mutex::new(camera));
    // if let Ok(c) = camera_mutex.lock() {
    //     let _sensor = c.sensor();
    //     if let Err(e) = _sensor.init_status() {
    //         log::error!("{}", e);
    //     }
    //     // if let Err(e) = _sensor.set_framesize(framesize) {
    //     //     log::error!("{}", e);
    //     // }
    // }
    let wifi = init_wifi(
        CONFIG.wifi_ssid,
        CONFIG.wifi_psk,
        &mut peripherals.modem,
        sysloop.clone(),
    )
    .await?;

    let _httpd = match init_http(camera_mutex) {
        Ok(h) => h,
        Err(e) => {
            error!("something happenned: {:?}", e);
            return Err(e);
        },
    };
    // let button = PinDriver::input(peripherals.pins.gpio34)?;
    let pir = PinDriver::input(peripherals.pins.gpio33);

    let main_loop = main_loop(peripherals.timer00, wifi, sysloop).await;
    drop(_httpd);
    main_loop
}

async fn main_loop(
    timer: impl Peripheral<P = impl Timer>,
    mut wifi: Box<EspWifi<'_>>,
    sysloop: EspSystemEventLoop,
) -> Result<()> {
    let mut delay_driver = TimerDriver::new(timer, &Default::default())?;

    'main: loop {
        match wifi.is_up() {
            Ok(false) | Err(_) => {
                warn!("WiFi died, attempting to reconnect...");
                let mut counter = 0;
                loop {
                    if wifi::connect(
                        CONFIG.wifi_ssid,
                        CONFIG.wifi_psk,
                        sysloop.clone(),
                        &mut wifi,
                    )
                    .await
                    .is_ok()
                    {
                        info!("WiFi reconnected successfully.");
                        break;
                    }
                    counter += 1;
                    warn!("Failed to connect to wifi, attempt {}", counter);

                    // If we fail to connect for long enough, reset the damn processor
                    if counter > 10 {
                        break 'main;
                    }
                }
            }
            _ => {}
        }

        delay_driver.delay(1000).await?;
    }

    bail!("Something went horribly wrong!!!")
}

