use std::marker::PhantomData;
use std::net::Ipv4Addr;
use std::thread;
use std::time::Duration;

use embedded_graphics::geometry::AnchorPoint;
use embedded_graphics::primitives::*;

use embedded_hal::digital;
use embedded_layout::{
    layout::linear::{spacing, Horizontal, LinearLayout, Vertical},
    prelude::*,
    ViewGroup,
};

use embedded_layout::object_chain::Chain;

use esp_idf_hal::gpio::{self};
use esp_idf_hal::i2c::I2cDriver;



use esp_idf_sys::EspError;
use log::{info, warn};
use log::error;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use thiserror::Error;
use display_interface::DisplayError;
use embedded_graphics::prelude::*;
use embedded_graphics::mono_font::MonoTextStyle;
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::{prelude::*, Ssd1306, I2CDisplayInterface};
use embedded_graphics::{
    Drawable,
    mono_font::{ascii::FONT_6X12, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    text::{Baseline, Text},
};

use crate::{small_display, preludes::InfoReceiver, window::{LabeledText, InputStatsRow, LabeledTextBuilder}};

#[derive(Error, Debug)]
pub enum SmallDisplayError {
    #[error("DrawTarget Error: {0}")]
    Draw(String),

    #[error("DisplayError: {0:?}")]
    Display(DisplayError),

    #[error(transparent)]
    Esp(#[from] EspError),

    #[error("other: {0}")]
    Other(String),

    #[error("unknown error")]
    Unknown,
}

impl From<DisplayError> for SmallDisplayError {
    fn from(value: DisplayError) -> Self {
        SmallDisplayError::Display(value)
    }
}

type Sdi<'d> = I2CInterface<I2cDriver<'d>>;
type SmDisplay<'a> = Ssd1306<
    I2CInterface<esp_idf_hal::i2c::I2cDriver<'a>>,
    DisplaySize128x64,
    BufferedGraphicsMode<DisplaySize128x64>
>;


pub fn bld_interface(i2c: I2cDriver<'_>) -> Result<Sdi<'_>, SmallDisplayError> {
    Ok(I2CDisplayInterface::new(i2c))
}


#[derive(Copy, Clone, Debug)]
pub struct SmallDisplay<'d, DI, SIZE, MODE, COLOR> {
    display: Ssd1306<DI, SIZE, MODE>,
    text_style: MonoTextStyle<'d, COLOR>,
}

impl<'d, DI, SIZE> SmallDisplay<'d, DI, SIZE, BufferedGraphicsMode<SIZE>, BinaryColor>
where
    DI: WriteOnlyDataCommand,
    SIZE: DisplaySize,
{
    pub fn new(
        interface: DI,
        size: SIZE,
        rotation: DisplayRotation,
    ) -> Self where SIZE: DisplaySize {
        let display = Ssd1306::new(interface, size, rotation).into_buffered_graphics_mode();
        let text_style = *DEFAULT_TEXT_STYLE.lock();
        Self { display , text_style}
    }
    pub fn init(&mut self) -> Result<(), SmallDisplayError> {
        self.display.init().map_err(SmallDisplayError::Display)
    }
    pub fn flush(&mut self) -> Result<(), SmallDisplayError> {
        self.display.flush().map_err(SmallDisplayError::Display)
    }

    pub fn write_text(&mut self, text: &str, position: Point, baseline: Baseline) -> Result<Point, SmallDisplayError> {
        match Text::with_baseline(text, position, self.text_style, baseline).draw(&mut self.display) {
            Ok(p) => Ok(p),
            Err(e) => Err(SmallDisplayError::Draw(format!("{:?}", e))),
        }
    }

}


pub fn init_display(
    interface: Sdi<'_>,
    size: impl DisplaySize,
    rotation: DisplayRotation,
) -> anyhow::Result<Ssd1306<I2CInterface<I2cDriver<'_>>, impl DisplaySize, ssd1306::mode::BasicMode>> {
    let disp = Ssd1306::new(interface, size, rotation);
    Ok(disp)
}

type DefaultTextStyle<'a> = MonoTextStyle<'a, BinaryColor>;
type MsgBoxText<'a> = Text<'a, DefaultTextStyle<'a>>;

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
type StatusInfoGpio = Option<digital::PinState>;
#[derive(Clone, Debug)]
pub struct StatusInfo<'txt, C: PixelColor> {
    address: Ipv4Addr,
    button_state: StatusInfoGpio,
    motion_state: StatusInfoGpio,
    pub win: StatusWindow<'txt, C>,
    ip_string: String,
}

impl<'txt> Default for StatusInfo<'txt, BinaryColor> {
    fn default() -> Self {
        Self {
            address: Ipv4Addr::UNSPECIFIED,
            button_state: None,
            motion_state: None,
            win: StatusWindow::new(*DEFAULT_TEXT_STYLE.lock()),
            ip_string: String::new(),
        }
    }
}


impl<'txt> StatusInfo<'txt, BinaryColor> {
    pub fn new(address: Ipv4Addr, button_state: StatusInfoGpio, motion_state: StatusInfoGpio) -> Self {
        Self {
            address,
            button_state,
            motion_state,
            win: StatusWindow::new(*DEFAULT_TEXT_STYLE.lock()),
            ip_string: address.to_string(),
        }
    }

    pub fn update<D: DrawTarget<Color=BinaryColor>>(&'txt mut self, info_update: &InfoUpdate, target: &mut D) -> Result<(), <D as DrawTarget>::Error> {
        let mut win = self.win;
        warn!("display update");
        match info_update {
            InfoUpdate::Addr(address) => {
                self.set_address(address.to_owned());
                win.set_ip_text(&self.ip_string);
            },
            InfoUpdate::Button(l) => {
                self.button_state = Some(l.to_owned());
                win.set_button_text(self.button_state_as_str());
            },
            InfoUpdate::Motion(l) => {
                self.motion_state = Some(l.to_owned());
                win.set_motion_text(self.motion_state_as_str());
            },
            InfoUpdate::Msg(ref m) => info!("update: {}", m),
        }
        self.win.draw(target)
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

impl<'txt, C: PixelColor> StatusWindow<'txt, C> {
    pub fn new(style: MonoTextStyle<'static, C>) -> Self {
        let mut ip_row = LabeledTextBuilder::new("IP:", style)
            .with_text(LONGEST_IPV4_ADDR)
            .build();
        let mut input_row = InputStatsRow::new(style);

        input_row.align_to_mut(&ip_row, horizontal::Left, vertical::TopToBottom);
        let s = Self {
            ip: LinearLayout::horizontal(ip_row).arrange().into_inner(),
            inputs: input_row,
        };
        LinearLayout::vertical(s).with_spacing(spacing::FixedMargin(2)).arrange().into_inner()
    }

    pub fn set_ip_text(&mut self, text: &'txt str) {
        self.ip.set_text(text)
    }
    pub fn set_button_text(&mut self, text: &'txt str) {
        self.inputs.set_button_text(text)
    }
    pub fn set_motion_text(&mut self, text: &'txt str) {
        self.inputs.set_motion_text(text)
    }

}


pub async fn display_runner<'d>(interface: I2CInterface<I2cDriver<'d>>, rx: InfoReceiver) -> Result<(), SmallDisplayError> {
    info!("started display_runner!!!!!!!");
    // let character_style: MonoTextStyle<'_, BinaryColor> = *DEFAULT_TEXT_STYLE.lock();
    let mut display = init_display(interface, DisplaySize128x64, DisplayRotation::Rotate0).unwrap()
        .into_buffered_graphics_mode();
    display.init()?;


    let display_bounds = display.bounding_box();

    display.clear_buffer();
    display.flush()?;

    // let mut status_info = StatusInfo::default();
    // status_info.win.align_to_mut(&display_bounds, horizontal::Left, vertical::Top);
    // status_info.win.draw(&mut display)?;
    // display.flush()?;

    loop {
        let mut status_info = StatusInfo::default();
        status_info.win.align_to_mut(&display_bounds, horizontal::Left, vertical::Top);
        status_info.win.draw(&mut display)?;
        display.flush()?;
            // let mut status_info = status_info.clone();
        let info_update = match rx.recv() {
            Ok(x) => x,
            Err(e) => {
                error!("error: {:?}", e);
                return Err(SmallDisplayError::Other(format!("{}", e)));
            },
        };
        match info_update {
            InfoUpdate::Addr(ip) => {
                info!("addr: {}", ip);
            },
            InfoUpdate::Button(level) => {
                let btn_str = format!("{:?}", level);
                info!("btn: {}", btn_str);
            },
            InfoUpdate::Motion(level) => {
                let pir_str = format!("{:?}", level);
                info!("pir: {}", pir_str);
            },
            InfoUpdate::Msg(ref text) => {
                info!("disp: {}", text);
            },
        }
        status_info.update(&info_update, &mut display)?;
        display.flush()?;
    }
}
