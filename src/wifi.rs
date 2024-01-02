use anyhow::Result;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::peripheral,
    nvs::EspDefaultNvsPartition,
    timer::EspTaskTimerService,
    wifi::{AuthMethod, ClientConfiguration, Configuration},
};

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
use tokio::time::sleep;


pub async fn init_wifi<'a>(
    ssid: &str,
    pass: &str,
    modem: impl peripheral::Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
) -> Result<Box<EspWifi<'static>>> {
    let mut esp_wifi = EspWifi::new(
        modem,
        sysloop.clone(),
        Some(EspDefaultNvsPartition::take()?),
    )?;

    let mut counter = 0;

    loop {
        if connect(ssid, pass, sysloop.clone(), &mut esp_wifi)
            .await
            .is_ok()
        {
            break;
        }
        counter += 1;
        warn!("Failed to connect to wifi, try {}", counter);
    }

    Ok(Box::new(esp_wifi))
}

pub async fn connect(
    ssid: &str,
    pass: &str,
    sysloop: EspSystemEventLoop,
    esp_wifi: &mut EspWifi<'_>,
) -> Result<()> {
    if ssid.is_empty() {
        panic!("Missing WiFi name")
    }

    let auth_method = if pass.is_empty() {
        info!("Wifi password is empty");
        AuthMethod::None
    } else {
        AuthMethod::WPA2Personal
    };

    let mut wifi = AsyncWifi::wrap(esp_wifi, sysloop, EspTaskTimerService::new()?)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))?;

    info!("Starting wifi...");

    wifi.start().await?;

    info!("Scanning...");

    let mut ap_infos = wifi.scan().await?.into_iter();

    let ours = ap_infos.find(|a| a.ssid == ssid);

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

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.into(),
        password: pass.into(),
        channel,
        auth_method,
        ..Default::default()
    }))?;

    info!("Connecting wifi...");

    wifi.connect().await?;

    info!("Waiting for DHCP lease...");

    wifi.wait_netif_up().await?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

    info!("Wifi DHCP info: {:?}", ip_info);

    Ok(())
}
