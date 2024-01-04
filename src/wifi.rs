use anyhow::Result;
use embedded_svc::wifi::AccessPointInfo;
use esp_idf_svc::{
    wifi::{ClientConfiguration, Configuration},
};
use flume::Sender;
use crate::{ntp::ntp_sync};
use crate::peripherals::{take_gpio12_output, take_gpio13_output};

use log::{info, warn};
use crate::preludes::*;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_svc::wifi::{AsyncWifi, EspWifi};
use esp_idf_sys::{
    esp_wifi_clear_ap_list, wifi_prov_event_handler_t, wifi_prov_mgr_config_t,
    wifi_prov_mgr_deinit, wifi_prov_mgr_init, wifi_prov_mgr_is_provisioned,
    wifi_prov_mgr_start_provisioning, wifi_prov_mgr_wait, wifi_prov_scheme_ble,
    wifi_prov_security_WIFI_PROV_SECURITY_1,
};
use std::{
    ffi::{c_void, CString},
    ptr::null_mut,
    thread,
};
// use tokio::time::sleep;



fn prov_led_blink() -> Result<()> {
    let mut count = 0;
    let mut led1 = take_gpio12_output();
    let mut led2 = take_gpio13_output();

    loop {
        FreeRtos::delay_ms(10);
        count += 1;
        let r = count % 30;
        if r > 15 {
            led1.set_high()?;
            led2.set_low()?;
        } else {
            led1.set_low()?;
            led2.set_high()?;
        }
    }
}

pub fn wifi_prov(wifi: &mut EspWifi) -> Result<()> {
    let pop = CString::new("abcd1234")?;
    let pop_ptr = pop.as_ptr() as *const c_void;

    thread::spawn(prov_led_blink);
    wifi.start()?;

    unsafe {
        let config = wifi_prov_mgr_config_t {
            scheme: wifi_prov_scheme_ble,
            scheme_event_handler: wifi_prov_event_handler_t {
                event_cb: None,
                user_data: null_mut(),
            },
            app_event_handler: wifi_prov_event_handler_t {
                event_cb: None,
                user_data: null_mut(),
            },
        };
        esp!(wifi_prov_mgr_init(config))?;
        let name = wifi.sta_netif().get_mac()?;
        let name = format!("PROV_MyFun_{}", hex::encode(name));
        let name = CString::new(name)?;

        esp!(wifi_prov_mgr_start_provisioning(
            wifi_prov_security_WIFI_PROV_SECURITY_1,
            pop_ptr,
            name.as_ptr(),
            null_mut(),
        ))?;
        wifi_prov_mgr_wait();
        wifi_prov_mgr_deinit();

        Ok(())
    }
}

pub fn prov_check() -> Result<bool> {
    unsafe {
        let mut provisioned = false;
        wifi_prov_mgr_is_provisioned(&mut provisioned);
        info!("provisioned: {provisioned}");
        Ok(provisioned)
    }
}

pub async fn initial_wifi_connect(wifi: &mut AsyncWifi<EspWifi<'static>>, tx: Sender<String>) -> Result<AccessPointInfo> {
    tx.send("initial_wifi".to_owned())?;
    wifi.start().await?;

    // let scan_result = wifi_scan(wifi).await?;
    match wifi_scan(wifi).await {
        Ok(ap) => {
            info!("Found configured access point {} on channel {}", ap.ssid, ap.channel);
            let mut ssid: heapless::String<32> = heapless::String::new();
            ssid.push_str(CONFIG.wifi_ssid).unwrap();
            let mut psk: heapless::String<64> = heapless::String::new();
            psk.push_str(CONFIG.wifi_psk).unwrap();
            wifi.set_configuration(&Configuration::Client(ClientConfiguration {
                ssid,
                password: psk,
                channel: Some(ap.channel),
                auth_method: ap.auth_method,
                ..Default::default()
            }))?;
            wifi.connect().await?;
            wifi.wait_netif_up().await?;
            let ip = wifi.wifi().sta_netif().get_ip_info()?;
            tx.send(ip.ip.to_string())?;
            info!("Connected to Wi-fi, now trying setting time from ntp.");
            ntp_sync()?;

            Ok(ap)

        },
        Err(e) => Err(e),
    }
}

pub async fn app_wifi_loop(mut wifi: AsyncWifi<EspWifi<'static>>, tx: Sender<String>) -> Result<()> {
    let mut count = 0u8;
    let mut fail_count = 0u8;

    warn!("wifi_loop");
    tx.send("msg".to_owned())?;
    initial_wifi_connect(&mut wifi, tx.clone()).await?;

    loop {
        // sleep(Duration::from_secs(10)).await;
        thread::sleep(Duration::from_secs(10));
        count += 1;

        if count == 8 {
            count = 0;
            // Wi-Fi driver will fault when trying scanning while connected to AP
            // if let Err(e) = wifi_scan(&mut wifi).await {
            //     error!("wifi_scan: {}", e);
            // }

            if fail_count > 0 {
                tx.send("msg".to_owned())?;

                info!("Network failure detected, try re-connecting...");
                wifi.disconnect().await?;
                wifi.stop().await?;
                initial_wifi_connect(&mut wifi, tx.clone()).await?;
                info!("Connected to Wi-fi, now trying setting time from ntp.");
                ntp_sync()?;
            }
        }

        if count >= 6 && fail_count > 0 {
            tx.send("msg".to_owned())?;
            if let Err(e) = ntp_sync() {
                error!("ntp_sync: {}", e);
                fail_count += 1;
            } else {
                fail_count = 0;
            };
        }
    }
}

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
}


pub async fn wifi_scan<'a>(wifi: &'a mut AsyncWifi<EspWifi<'static>>) -> Result<AccessPointInfo> {
    esp!(unsafe { esp_wifi_clear_ap_list() })?;
    let ssid = CONFIG.wifi_ssid;

    if let Ok((scan, _)) = wifi.scan_n::<32>().await {
        for ap in scan.into_iter() {
            if ap.ssid == ssid {
                return Ok(ap);
            }
        }
    }
    Err(anyhow!("couldn't find ssid"))
}

pub type MacList = HeaplessVec<[u8; 6], 32>;
