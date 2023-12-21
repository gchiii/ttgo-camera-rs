// use anyhow::Result;
// use esp_idf_svc::{
//     eventloop::EspSystemEventLoop,
//     hal::peripheral,
//     nvs::EspDefaultNvsPartition,
//     timer::EspTaskTimerService,
//     wifi::{AsyncWifi, AuthMethod, ClientConfiguration, Configuration, EspWifi},
// };

// use log::{info, warn};

// fn ping(ip: ipv4::Ipv4Addr) -> Result<()> {
//     info!("About to do some pings for {:?}", ip);

//     let ping_summary = ping::EspPing::default().ping(ip, &Default::default())?;
//     if ping_summary.transmitted != ping_summary.received {
//         bail!("Pinging IP {} resulted in timeouts", ip);
//     }

//     info!("Pinging done");

//     Ok(())
// }



// fn wifi(
//     modem: impl peripheral::Peripheral<P = esp_idf_svc::hal::modem::Modem> + 'static,
//     sysloop: EspSystemEventLoop,
//     nvs: Option<EspDefaultNvsPartition>,
// ) -> Result<Box<EspWifi<'static>>> {
//     let app_config = CONFIG;
//     let ssid: &str = CONFIG.wifi_ssid;
//     let pass = CONFIG.wifi_psk;

//     info!("ssid = {} pw = {}", ssid, pass);

//     let mut esp_wifi = EspWifi::new(modem, sysloop.clone(), nvs)?;

//     let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sysloop)?;

//     wifi.set_configuration(&esp_idf_svc::wifi::Configuration::Client(ClientConfiguration::default()))?;

//     info!("Starting wifi...");

//     wifi.start()?;

//     info!("Scanning...");

//     let mut ap_infos = wifi.scan()?;

//     for ap in &ap_infos {
//         info!("ap {:?}", ap);
//     }
//     let mut ours = ap_infos.into_iter().find(|a| a.ssid == ssid);
//     if ours.is_none() {
//         ap_infos = wifi.scan()?;
//         for ap in &ap_infos {
//             info!("ap {:?}", ap);
//         }
//         ours = ap_infos.into_iter().find(|a| a.ssid == ssid);
//     }

//     let channel = if let Some(ours) = ours {
//         info!(
//             "Found configured access point {} on channel {}",
//             ssid, ours.channel
//         );
//         Some(ours.channel)
//     } else {
//         info!(
//             "Configured access point {} not found during scanning, will go with unknown channel",
//             ssid
//         );
//         None
//     };
//     // let client_config = ClientConfiguration {
//     //     ssid: ssid.into(),
//     //     password: pass.into(),
//     //     channel,
//     //     ..Default::default()
//     // };
//     // wifi.set_configuration(&Configuration::Client(client_config))?;
//     wifi.set_configuration(&esp_idf_svc::wifi::Configuration::Mixed(
//         ClientConfiguration {
//             ssid: ssid.into(),
//             password: pass.into(),
//             channel,
//             ..Default::default()
//         },
//         AccessPointConfiguration {
//             ssid: "aptest".into(),
//             channel: channel.unwrap_or(1),
//             ..Default::default()
//         },
//     ))?;

//     info!("Connecting wifi...");

//     wifi.connect()?;

//     info!("Waiting for DHCP lease...");

//     if let Err(e) = wifi.wait_netif_up() {
//         error!("wifi error: {:?}", e);
//         wifi.connect()?;
//     }
//     wifi.wait_netif_up()?;

//     let ip_info = wifi.wifi().sta_netif().get_ip_info()?;

//     info!("Wifi DHCP info: {:?}", ip_info);

//     ping(ip_info.subnet.gateway)?;

//     Ok(Box::new(esp_wifi))
// }
