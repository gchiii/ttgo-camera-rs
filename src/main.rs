
use anyhow::{bail, Result, anyhow};

use edge_executor::{LocalExecutor, Executor};
use embassy_futures::yield_now;
use embedded_graphics::{geometry::Point, text::{Baseline, Text}, pixelcolor::BinaryColor, mono_font::{MonoTextStyleBuilder, ascii::FONT_6X12}, Drawable};
use embedded_svc::ipv4::{IpInfo, Subnet, Mask};

use esp_camera_rs::Camera;

use esp_idf_sys::EspError;
use flume::Sender;
use futures::FutureExt;
use ssd1306::{rotation::DisplayRotation, size::{DisplaySize128x64}, prelude::I2CInterface, mode::DisplayConfig};


use std::{
    time::{Instant, Duration},
    sync::{Arc, Mutex}, net::Ipv4Addr,
};


use esp_idf_hal::{reset::{ResetReason, WakeupReason}, i2c::{I2cDriver}};
use esp_idf_svc::{
    hal::{
        peripherals::Peripherals,
        peripheral::Peripheral,
        timer::{TimerDriver}
    },
    io::Write,
    eventloop::{EspSystemEventLoop},
    wifi::{EspWifi, AsyncWifi},
    http::server::EspHttpServer,
};
use log::*;

// use esp-camera-rs::Camera;

// mod app;
// mod ble;
// mod build_env;
// mod crypto;
// mod http;
// mod key_inspect;
// mod mqtt;
mod ntp;
mod peripherals;
mod preludes;
// mod proto;
mod wifi;

mod small_display;
// mod esp_camera;
// use crate::esp_camera::Camera;

// use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use crate::{wifi::{app_wifi_loop, initial_wifi_connect}, peripherals::{create_timer_driver_00, take_i2c, SYS_LOOP, PERIPHERALS, ESP_TASK_TIMER_SVR, create_esp_wifi}};
use crate::small_display::*;


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




fn init_http(cam: Arc<Mutex<Camera>>, tx: Sender<String>) -> Result<EspHttpServer> {
    let httpd_config = esp_idf_svc::http::server::Configuration {
        session_timeout: Duration::from_secs(5*50),
        uri_match_wildcard: true,
        ..Default::default()
    };
    let mut server = EspHttpServer::new(&httpd_config)?;

    server.fn_handler("/", esp_idf_svc::http::Method::Get, move |request| {
        let mut time = Instant::now();
        info!("handling request");
        if let Err(e) = tx.send("handling request".to_owned()) {
            error!("trouble sending: {}", e);
        }
        let lock = cam.lock().unwrap(); // If a thread gets poisoned we're just dead anyways
        let _sensor = lock.sensor();

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

async fn display_runner<'d>(interface: I2CInterface<I2cDriver<'d>>, rx: flume::Receiver<String>) -> Result<()> {
    warn!("started display_runner!!!!!!!");
    let mut display = init_display(interface, DisplaySize128x64, DisplayRotation::Rotate0).unwrap()
        .into_buffered_graphics_mode();
    let _ = display.init();
    draw_shapes(&mut display);
    if let Err(e) = display.flush() {
        error!("error: {:?}", e);
    }
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X12)
        .text_color(BinaryColor::On)
        .build();

    warn!("blah");
    loop {
        let _ = flume::Selector::new()
            .recv(&rx, |thing| {
                match thing {
                    Ok(text) => {
                        warn!("disp: {}", text);
                        if let Err(e) = Text::with_baseline(text.as_str(), Point::zero(), text_style, Baseline::Top).draw(&mut display) {
                            error!("error: {:?}", e);
                        }
                    },
                    Err(e) => {
                        error!("error: {:?}", e);
                        return Err(anyhow!("display: {}", e));
                    },
                };
                Ok(())
            })
            .wait();
        // yield_now().await;
    }
    // Ok(())
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

    let i2c = take_i2c();
    let sd_iface = bld_interface(i2c)?;
    let (tx, rx) = flume::unbounded::<String>();

    let mut wifi: EspWifi<'static> = create_esp_wifi();
    let mut mywifi: AsyncWifi<EspWifi<'static>> = AsyncWifi::wrap(wifi, SYS_LOOP.clone(), ESP_TASK_TIMER_SVR.clone()).unwrap();

    let p = PERIPHERALS.clone();
    let mut p = p.lock();
    let cam_sda = unsafe { &mut p.pins.gpio18.clone_unchecked()};
    let cam_scl = unsafe { &mut p.pins.gpio23.clone_unchecked()};
    let cam_pwdn = unsafe { &mut p.pins.gpio26.clone_unchecked()};
    let pin_xclk = unsafe { &mut p.pins.gpio4.clone_unchecked()};
    let pin_d0 = unsafe { &mut p.pins.gpio34.clone_unchecked()};
    let pin_d1 = unsafe { &mut p.pins.gpio13.clone_unchecked()};
    let pin_d2 = unsafe { &mut p.pins.gpio14.clone_unchecked()};
    let pin_d3 = unsafe { &mut p.pins.gpio35.clone_unchecked()};
    let pin_d4 = unsafe { &mut p.pins.gpio39.clone_unchecked()};
    let pin_d5 = unsafe { &mut p.pins.gpio12.clone_unchecked()};
    let pin_d6 = unsafe { &mut p.pins.gpio15.clone_unchecked()};
    let pin_d7 = unsafe { &mut p.pins.gpio36.clone_unchecked()};
    let pin_vsync = unsafe { &mut p.pins.gpio5.clone_unchecked()};
    let pin_href = unsafe { &mut p.pins.gpio27.clone_unchecked()};
    let pin_pclk = unsafe { &mut p.pins.gpio25.clone_unchecked()};
    drop(p);

    let camera = Camera::new(
        Some(cam_pwdn.into_ref().map_into()),
        None,
        pin_xclk,
        pin_d0,
        pin_d1,
        pin_d2,
        pin_d3,
        pin_d4,
        pin_d5,
        pin_d6,
        pin_d7,
        pin_vsync,
        pin_href,
        pin_pclk,
        Some(cam_sda.into_ref().map_into()),
        Some(cam_scl.into_ref().map_into()),
    )?;
    let camera_mutex = Arc::new(Mutex::new(camera));
    let _http = match init_http(camera_mutex, tx.clone()) {
        Err(e) => {
            error!("init_http: {}", e);
            return Err(e);
        }
        Ok(h) => h,
    };

    let ex: Executor<'_, 64> = edge_executor::Executor::default();
    edge_executor::block_on( async move {
        let _ = futures::executor::block_on(initial_wifi_connect(&mut mywifi, tx.clone()));
        let _disp_task = ex.spawn(display_runner(sd_iface, rx));
        let _wifi_loop = ex.spawn( app_wifi_loop(mywifi, tx.clone()) );
        while ex.try_tick() {
            std::thread::sleep(Duration::from_secs(1));
        }

    } );

    drop(_http);
    Ok(())
    // executor.spawn(display_runner(sd_iface, rx)).detach();
    // let main_task = executor.spawn(async_main(tx));
    // edge_executor::block_on(main_task)
    // edge_executor::block_on(executor.run(async_main(tx)))
    // edge_executor::block_on(async_main())
}


