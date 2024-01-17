
use anyhow::Result as AnyResult;
use edge_executor::Executor;
use esp_camera_rs::Camera;
use esp_idf_sys::EspError;
use preludes::InfoSender;
use std::{
    time::{Instant, Duration},
    sync::{Arc, Mutex},
};



use esp_idf_hal::{reset::{ResetReason, WakeupReason}, gpio::{PinDriver, self, InputPin, Input}};
use esp_idf_svc::{
    hal::peripheral::Peripheral,
    io::Write,
    wifi::{EspWifi, AsyncWifi},
    http::server::EspHttpServer, eventloop::{EspBackgroundEventLoop, EspBackgroundSubscription},
};
use log::*;
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::{prelude::*, Ssd1306};

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
mod wifi;
mod small_display;
mod window;
mod screen;

use crate::{preludes::*, event::EventLoopMessage};

use crate::{wifi::{app_wifi_loop, initial_wifi_connect}, peripherals::{take_i2c, SYS_LOOP, PERIPHERALS, ESP_TASK_TIMER_SVR, create_esp_wifi}};
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
// PIR: AS312   - on GPIO 19
// UART chip: CP2104
// Charging chip: IP5306 I2C
// Camera: OV2640
// Camera Resolution: 2 Megapixel
// PIR input GPIO 19
// BUTTON input GPIO 0


mod event {
    use esp_idf_svc::eventloop::{
        EspEventFetchData, EspEventPostData, EspTypedEventDeserializer, EspTypedEventSerializer,
        EspTypedEventSource,
    };
    // use esp_idf_sys::libc;

    #[derive(Copy, Clone, Debug)]
    pub struct EventLoopMessage(u8);

    impl EventLoopMessage {
        pub fn new(data: u8) -> Self {
            Self(data)
        }
    }

    impl EspTypedEventSource for EventLoopMessage {
        fn source() -> *const std::ffi::c_char {
            b"DEMO-SERVICE\0".as_ptr() as *const _
        }
    }

    impl EspTypedEventSerializer<EventLoopMessage> for EventLoopMessage {
        fn serialize<R>(
            event: &EventLoopMessage,
            f: impl for<'a> FnOnce(&'a EspEventPostData) -> R,
        ) -> R {
            f(&unsafe { EspEventPostData::new(Self::source(), Self::event_id(), event) })
        }
    }

    impl EspTypedEventDeserializer<EventLoopMessage> for EventLoopMessage {
        fn deserialize<R>(
            data: &EspEventFetchData,
            f: &mut impl for<'a> FnMut(&'a EventLoopMessage) -> R,
        ) -> R {
            f(unsafe { data.as_payload() })
        }
    }
}

fn init_eventloop() -> Result<(EspBackgroundEventLoop, EspBackgroundSubscription<'static>), EspError> {
    info!("About to start a background event loop");
    let eventloop = EspBackgroundEventLoop::new(&Default::default())?;

    info!("About to subscribe to the background event loop");
    let subscription = eventloop.subscribe(|message: &EventLoopMessage| {
        info!("Got event from the event loop: {:?}", message);
    })?;

    Ok((eventloop, subscription))
}


fn init_http(cam: Arc<Mutex<Camera>>, tx: InfoSender) -> AnyResult<EspHttpServer> {
    let httpd_config = esp_idf_svc::http::server::Configuration {
        session_timeout: Duration::from_secs(5*50),
        uri_match_wildcard: true,
        ..Default::default()
    };
    let mut server = EspHttpServer::new(&httpd_config)?;

    server.fn_handler("/", esp_idf_svc::http::Method::Get, move |request| {
        let mut time = Instant::now();
        info!("handling request");
        if let Err(e) = tx.send(InfoUpdate::Msg("handling request".to_owned())) {
            error!("trouble sending: {}", e);
        }
        match cam.lock() {
            Ok(lock) => {
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

            },
            Err(e) => {
                error!("something terrible: {:?}", e);
            },
        }

        Ok(())
    })?;

    Ok(server)
}


async fn motion_task<P>(motion: PinDriver<'static, P, gpio::Input>, tx: InfoSender) -> AnyResult<()>
where
    P: InputPin,
{
    warn!("pir_task");
    let mut level = motion.get_level();
    tx.send(InfoUpdate::Motion(level.into()))?;
    loop {
        std::thread::sleep(Duration::from_micros(250));
        let tmp_level = motion.get_level();
        if level != tmp_level {
            level = tmp_level;
            if let Err(e) = tx.send(InfoUpdate::Motion(level.into())) {
                error!("motion tx.send: {:?}", e);
            }
        }
    }
    // Ok(())
}

async fn button_task<P>(button: PinDriver<'_, P, Input>, tx: InfoSender) -> AnyResult<()>
where
    P: InputPin,
{
    let mut level = button.get_level();
    tx.send(InfoUpdate::Button(level.into()))?;
    loop {
        std::thread::sleep(Duration::from_micros(150));
        let tmp_level = button.get_level();
        if level != tmp_level {
            level = tmp_level;
            if let Err(e) = tx.send(InfoUpdate::Button(level.into())) {
                error!("button tx.send: {:?}", e);
            }
        }
        // if let Err(e) = tx.send(InfoUpdate::Button(button.get_level().into())) {
        //     error!("button.wait_for_low: {:?}", e);
        // }
    }
}

fn main() -> AnyResult<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();
    peripherals::patch_eventfd();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();
    log::info!("Hello, world!");

    // let (mut eventloop, _subscription) = init_eventloop().unwrap();

    let reset_reason = ResetReason::get();
    info!("Last reset was due to {:#?}", reset_reason);
    let wakeup_reason = WakeupReason::get();
    info!("Last wakeup was due to {:#?}", wakeup_reason);

    let i2c: esp_idf_hal::i2c::I2cDriver<'static> = take_i2c();
    let sd_iface: I2CInterface<esp_idf_hal::i2c::I2cDriver<'static>> = bld_interface(i2c)?;

    // let (tx, rx) = flume::unbounded::<InfoUpdate>();
    let (tx, rx) = crossbeam_channel::unbounded::<InfoUpdate>();

    let wifi: EspWifi<'static> = create_esp_wifi();
    let mut mywifi: AsyncWifi<EspWifi<'static>> = AsyncWifi::wrap(wifi, SYS_LOOP.clone(), ESP_TASK_TIMER_SVR.clone()).unwrap();

    let p = PERIPHERALS.clone();
    let mut p = p.lock();
    // #[cfg(feature="USE_CAMERA")]
    // {
        let cam_sda = unsafe { &mut p.pins.gpio18.clone_unchecked()};
        let cam_scl = unsafe { &mut p.pins.gpio23.clone_unchecked()};
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
    // }

    #[cfg(feature="MIC")]
    {
        let pin_mic_ws = unsafe { &mut p.pins.gpio32.clone_unchecked()};
        let pin_mic_sck = unsafe { &mut p.pins.gpio26.clone_unchecked()};
        let pin_mic_sd = unsafe { &mut p.pins.gpio33.clone_unchecked()};
    }

    #[cfg(feature="IP5306")]
    {
        // IP5306
        let ip5306_led1 = unsafe { &mut p.pins.gpio22.clone_unchecked()};
        let ip5306_led2 = unsafe { &mut p.pins.gpio21.clone_unchecked()};
        let ip5306_led3 = unsafe { &mut p.pins.gpio2.clone_unchecked()};
    }
    let pir_pin = unsafe {p.pins.gpio19.clone_unchecked()};
    let pb_pin = unsafe {p.pins.gpio0.clone_unchecked()};
    drop(p);

    let pir  = PinDriver::input(pir_pin)?;
    let mut push_button = PinDriver::input(pb_pin)?;
    push_button.set_pull(gpio::Pull::Up)?;

    // #[cfg(feature="USE_CAMERA")]
    // {
        let camera = Camera::new(
            None,
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
    // }
    let disp: Ssd1306<I2CInterface<esp_idf_hal::i2c::I2cDriver<'static>>, DisplaySize128x64, ssd1306::mode::BasicMode> = Ssd1306::new(sd_iface, DisplaySize128x64, DisplayRotation::Rotate0);
    let mut display: Ssd1306<I2CInterface<esp_idf_hal::i2c::I2cDriver<'static>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>> = disp.into_buffered_graphics_mode();
    let _ = display.init();
    display.clear_buffer();
    let _ = display.flush();

    let ex: Executor<'_, 64> = edge_executor::Executor::default();
    edge_executor::block_on( async move {

        let _ = futures::executor::block_on(initial_wifi_connect(&mut mywifi, tx.clone()));
        let button_task = ex.spawn(button_task(push_button, tx.clone()));
        let motion_task = ex.spawn(motion_task(pir, tx.clone()));
        let wifi_loop_task = ex.spawn( app_wifi_loop(mywifi, tx.clone()) );
        let display_task = ex.spawn(display_runner(display, rx));
        loop {
            while ex.try_tick() {
                std::thread::sleep(Duration::from_micros(250));
            }
            if button_task.is_finished() {
                error!("button task done! {:?}", button_task);
            }
            if motion_task.is_finished() {
                error!("motion task done");
            }
            if wifi_loop_task.is_finished() {
                error!("wifi task done");
            }
            if display_task.is_finished() {
                error!("display task done");
            }
            error!("why are we done with the threads?");
        }
        // #[cfg(feature="USE_CAMERA")]
        drop(_http);

    } );

    Ok(())
}


