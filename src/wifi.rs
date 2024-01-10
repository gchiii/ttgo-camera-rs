use anyhow::Result;
use embedded_svc::wifi::AccessPointInfo;
use esp_idf_svc::{
    wifi::{ClientConfiguration, Configuration},
};
use flume::Sender;
use crate::{ntp::ntp_sync, small_display::InfoUpdate};
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


pub async fn initial_wifi_connect(wifi: &mut AsyncWifi<EspWifi<'static>>, tx: InfoSender) -> Result<AccessPointInfo> {
    tx.send(InfoUpdate::Msg("initial_wifi".to_owned()))?;
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
            warn!("ip: {:?}", ip);
            tx.send(InfoUpdate::Addr(ip.ip))?;
            info!("Connected to Wi-fi, now trying setting time from ntp.");
            ntp_sync()?;

            Ok(ap)

        },
        Err(e) => Err(e),
    }
}

pub async fn app_wifi_loop(mut wifi: AsyncWifi<EspWifi<'static>>, tx: InfoSender) -> Result<()> {
    let mut count = 0u8;
    let mut fail_count = 0u8;

    warn!("wifi_loop");
    tx.send(InfoUpdate::Msg("msg".to_owned()))?;
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
                tx.send(InfoUpdate::Msg("msg".to_owned()))?;

                info!("Network failure detected, try re-connecting...");
                wifi.disconnect().await?;
                wifi.stop().await?;
                initial_wifi_connect(&mut wifi, tx.clone()).await?;
                info!("Connected to Wi-fi, now trying setting time from ntp.");
                ntp_sync()?;
            }
        }

        if count >= 6 && fail_count > 0 {
            tx.send(InfoUpdate::Msg("msg".to_owned()))?;
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
