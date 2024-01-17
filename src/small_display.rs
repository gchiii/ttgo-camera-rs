

use display_interface::DisplayError;
// use display_interface::DisplayError;

use embedded_layout::{
    prelude::*,
};
use esp_idf_hal::i2c::I2cDriver;
use esp_idf_sys::EspError;
use log::{info, error};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use thiserror::Error;
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
use crate::{preludes::*, screen::{StatusInfo, StatusWindow}};


#[derive(Error, Debug)]
pub enum SmallDisplayError {
    #[error("DrawTarget Error: {0}")]
    Draw(String),

    #[error("{0:?}")]
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


pub fn bld_interface(i2c: I2cDriver<'static>) -> Result<Sdi<'static>, SmallDisplayError> {
    Ok(I2CDisplayInterface::new(i2c))
}


#[derive(Copy, Clone, Debug)]
pub struct SmallDisplay<'d, DI, SIZE, MODE, COLOR> {
    display: Ssd1306<DI, SIZE, MODE>,
    text_style: MonoTextStyle<'d, COLOR>,
}
static DEFAULT_TEXT_STYLE: Lazy<Mutex<MonoTextStyle<'static, BinaryColor>>> = Lazy::new(|| {
    Mutex::new(MonoTextStyleBuilder::new()
        .font(&FONT_6X12)
        .text_color(BinaryColor::On)
        .background_color(BinaryColor::Off)
        .build()
    )
});

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
    pub fn init(&mut self) {
        let _ = self.display.init();//?;//.map_err(|e| SmallDisplayError::Display(e))
    }
    pub fn flush(&mut self) {
        let _ = self.display.flush();//.map_err(SmallDisplayError::Display)
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

// type DefaultTextStyle<'a> = MonoTextStyle<'a, BinaryColor>;
// type MsgBoxText<'a> = Text<'a, DefaultTextStyle<'a>>;



pub async fn display_runner(mut display: Ssd1306<I2CInterface<esp_idf_hal::i2c::I2cDriver<'static>>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>, rx: InfoReceiver) -> Result<(), SmallDisplayError> {
    info!("started display_runner!!!!!!!");
    let character_style = *DEFAULT_TEXT_STYLE.lock();
    // let mut display = init_display(interface, DisplaySize128x64, DisplayRotation::Rotate0).unwrap()
    //     .into_buffered_graphics_mode();
    // let _ = display.init();


    let display_bounds = display.bounding_box();

    display.clear_buffer();
    let _ = display.flush();

    let mut status_info = StatusInfo::default();
    loop {
        let x = rx.recv();
            // let mut status_info = status_info.clone();
        match x {
            Ok(info_update) => {
                status_info.update(&info_update);
                let mut win = StatusWindow::from(character_style, &status_info);
                win.align_to_mut(&display_bounds, horizontal::Left, vertical::Top);
                if let Err(e) = win.draw(&mut display) {
                    error!("oops!: {:?}", e);
                }
                let _ = &display.flush();
                info!("flush");
            },
            Err(e) => {
                error!("error: {:?}", e);
                return Err(SmallDisplayError::Other(format!("{}", e)));
            },
        };
        // match info_update {
        //     InfoUpdate::Addr(ip) => {
        //         info!("addr: {}", ip);
        //     },
        //     InfoUpdate::Button(level) => {
        //         let btn_str = format!("{:?}", level);
        //         info!("btn: {}", btn_str);
        //     },
        //     InfoUpdate::Motion(level) => {
        //         let pir_str = format!("{:?}", level);
        //         info!("pir: {}", pir_str);
        //     },
        //     InfoUpdate::Msg(ref text) => {
        //         info!("disp: {}", text);
        //     },
        // }
    }
}
