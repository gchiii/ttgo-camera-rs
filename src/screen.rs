
use std::net::Ipv4Addr;

use embedded_hal::digital;
use embedded_layout::{
    layout::linear::{spacing, LinearLayout},
    prelude::*,
    ViewGroup,
};


use log::{info, warn};
use once_cell::sync::Lazy;
use parking_lot::Mutex;

use embedded_graphics::prelude::*;
use embedded_graphics::mono_font::MonoTextStyle;


use embedded_graphics::{
    Drawable,
    mono_font::{ascii::FONT_6X12, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
};

use crate::{window::{LabeledText, InputStatsRow, LabeledTextBuilder}};


static DEFAULT_TEXT_STYLE: Lazy<Mutex<MonoTextStyle<'static, BinaryColor>>> = Lazy::new(|| {
    Mutex::new(MonoTextStyleBuilder::new()
        .font(&FONT_6X12)
        .text_color(BinaryColor::On)
        .background_color(BinaryColor::Off)
        .build()
    )
});


#[derive(Clone, Debug)]
pub enum InfoUpdate {
    Addr(Ipv4Addr),
    Button(digital::PinState),
    Motion(digital::PinState),
    Msg(String),
}


// screen width 128
// screen height 64
// top line is the ip address
// next line has PIR and Button
// use the rest for messages

const LONGEST_IPV4_ADDR: &str = "255.255.255.255";
pub type StatusInfoGpio = Option<digital::PinState>;
#[derive(Clone, Debug)]
pub struct StatusInfo {
    address: Ipv4Addr,
    button_state: StatusInfoGpio,
    motion_state: StatusInfoGpio,
    ip_string: String,
}


impl Default for StatusInfo {
    fn default() -> Self {
        Self {
            address: Ipv4Addr::UNSPECIFIED,
            button_state: None,
            motion_state: None,
            ip_string: String::new(),
        }
    }
}

#[allow(unused)]
impl StatusInfo {
    pub fn new(address: Ipv4Addr, button_state: StatusInfoGpio, motion_state: StatusInfoGpio) -> Self {
        Self {
            address,
            button_state,
            motion_state,
            ip_string: address.to_string(),
        }
    }

    pub fn update(&mut self, info_update: &InfoUpdate) {
        warn!("display update");
        match info_update {
            InfoUpdate::Addr(address) => {
                println!("ip: {}", address);
                self.set_address(address.to_owned())
            },
            InfoUpdate::Button(l) => {
                println!("button");
                self.button_state = Some(l.to_owned());
            },
            InfoUpdate::Motion(l) => {
                println!("motion");
                self.motion_state = Some(l.to_owned());
            },
            InfoUpdate::Msg(ref m) => {
                println!("update: {}", m);
            },
        }
    }

    #[inline]
    pub fn gpio_status_as_str(gpio_stat: &StatusInfoGpio) -> &str {
        match gpio_stat {
            Some(digital::PinState::High) => "High",
            Some(digital::PinState::Low) => "Low",
            None => "None",
        }
    }
    #[inline]
    pub fn button_state_as_str(&self) -> &str {
        Self::gpio_status_as_str(&self.button_state)
    }
    #[inline]
    pub fn motion_state_as_str(&self) -> &str {
        Self::gpio_status_as_str(&self.motion_state)
    }

    pub fn address_as_str(&self) -> &str {
        self.ip_string.as_str()
    }

    pub fn set_address(&mut self, address: Ipv4Addr) {
        self.address = address;
        self.ip_string = address.to_string();
    }

    pub fn set_button_state(&mut self, button_state: StatusInfoGpio) {
        self.button_state = button_state;
    }

    pub fn set_motion_state(&mut self, motion_state: StatusInfoGpio) {
        self.motion_state = motion_state;
    }
}


#[derive(Clone, Copy, Debug, ViewGroup)]
pub struct StatusWindow<'txt, C: PixelColor> {
    ip: LabeledText<'txt, C>,
    inputs: InputStatsRow<'txt, C>,
}

impl<'txt> Default for StatusWindow<'txt, BinaryColor> {
    fn default() -> Self {
        let style = *DEFAULT_TEXT_STYLE.lock();
        let ip_row = LabeledTextBuilder::new("IP:", style)
            .with_text(LONGEST_IPV4_ADDR)
            .build();
        let mut input_row = InputStatsRow::default();

        input_row.align_to_mut(&ip_row, horizontal::Left, vertical::TopToBottom);
        let s = Self {
            ip: LinearLayout::horizontal(ip_row).arrange().into_inner(),
            inputs: input_row,
        };
        LinearLayout::vertical(s).with_spacing(spacing::FixedMargin(2)).arrange().into_inner()
    }
}

impl<'txt, C: PixelColor> StatusWindow<'txt, C> {

    fn new_ip_row(style: MonoTextStyle<'static, C>, address: &'txt str) -> LabeledText<'txt, C> {
        LabeledTextBuilder::new("IP:", style)
            .with_text(address)
            .build()
    }

    pub fn new(style: MonoTextStyle<'static, C>, address: &'txt str, button: &'txt StatusInfoGpio, motion: &'txt StatusInfoGpio) -> Self {
        let ip_row = Self::new_ip_row(style, address);
        let mut input_row = InputStatsRow::new(style, button, motion);

        input_row.align_to_mut(&ip_row, horizontal::Left, vertical::TopToBottom);
        let s = Self {
            ip: LinearLayout::horizontal(ip_row).arrange().into_inner(),
            inputs: input_row,
        };
        LinearLayout::vertical(s).with_spacing(spacing::FixedMargin(2)).arrange().into_inner()
    }

    pub fn from(style: MonoTextStyle<'static, C>, status_info: &'txt StatusInfo) -> Self {
        // let motion = &status_info.motion_state;
        // let ip_string = &status_info.ip_string;
        // let pin_state = status_info.button_state;
        Self::new(style,
            &status_info.ip_string,
            &status_info.button_state,
            &status_info.motion_state)
    }

    pub fn do_layout(&self) -> Self {
        LinearLayout::vertical(*self).with_spacing(spacing::FixedMargin(2)).arrange().into_inner()
    }

    pub fn set_ip_text(&mut self, text: &'txt str) {
        info!("set_ip_text: {}", text);
        self.ip = self.ip.with_text(text)
    }
    pub fn set_button_text(&mut self, button: &'txt StatusInfoGpio) {
        self.inputs = self.inputs.update_button(button);
    }
    pub fn set_motion_text(&mut self, motion: &'txt StatusInfoGpio) {
        self.inputs = self.inputs.update_motion(motion);
    }

}
