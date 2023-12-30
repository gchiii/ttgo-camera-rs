use embedded_graphics::primitives::*;
use esp_idf_hal::gpio::{InputPin, OutputPin};
use esp_idf_hal::i2c::{I2c, I2cDriver, I2cConfig};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::units::*;

use esp_idf_sys::EspError;
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

#[derive(Error, Debug)]
pub enum SmallDisplayError {
    #[error("DrawTarget Error: {0}")]
    Draw(String),

    #[error("DisplayError: {0:?}")]
    Display(DisplayError),

    #[error(transparent)]
    Esp(#[from] EspError),

    #[error("unknown error")]
    Unknown,
}

type Sdi<'d> = I2CInterface<I2cDriver<'d>>;

pub fn bld_interface<'d>(
    i2c: impl Peripheral<P = impl I2c> + 'd,
    sda: impl Peripheral<P = impl InputPin + esp_idf_hal::gpio::OutputPin> + 'd,
    scl: impl Peripheral<P = impl InputPin + OutputPin> + 'd,
) -> Result<Sdi<'d>, SmallDisplayError> {
    let config: esp_idf_hal::i2c::config::Config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c: I2cDriver<'d> = I2cDriver::<'d>::new(i2c, sda, scl, &config)?;
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
    pub fn bld_interface(
        i2c: impl Peripheral<P = impl I2c> + 'd,
        sda: impl Peripheral<P = impl InputPin + esp_idf_hal::gpio::OutputPin> + 'd,
        scl: impl Peripheral<P = impl InputPin + OutputPin> + 'd,
    ) -> Result<I2CInterface<I2cDriver<'d>>, SmallDisplayError> {
        let config: esp_idf_hal::i2c::config::Config = I2cConfig::new().baudrate(400.kHz().into());
        let i2c: I2cDriver<'d> = I2cDriver::<'d>::new(i2c, sda, scl, &config)?;
        Ok(I2CDisplayInterface::new(i2c))
    }

    pub fn new(
        interface: DI,
        size: SIZE,
        rotation: DisplayRotation,
    ) -> Self where SIZE: DisplaySize {
        let display = Ssd1306::new(interface, size, rotation).into_buffered_graphics_mode();
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X12)
            .text_color(BinaryColor::On)
            .build();
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

pub fn draw_shapes<D, E>(display: &mut D) where D: DrawTarget<Color = BinaryColor, Error = E>, E: std::fmt::Debug
{
    let yoffset = 8;

    let style = PrimitiveStyleBuilder::new()
        .stroke_width(1)
        .stroke_color(BinaryColor::On)
        .build();

    // screen outline
    // default display size is 128x64 if you don't pass a _DisplaySize_
    // enum to the _Builder_ struct
    Rectangle::new(Point::new(0, 0), Size::new(127, 31))
        .into_styled(style)
        .draw(display)
        .unwrap();

    // triangle
    Triangle::new(
        Point::new(16, 16 + yoffset),
        Point::new(16 + 16, 16 + yoffset),
        Point::new(16 + 8, yoffset),
    )
    .into_styled(style)
    .draw(display)
    .unwrap();

    // square
    Rectangle::new(Point::new(52, yoffset), Size::new_equal(16))
        .into_styled(style)
        .draw(display)
        .unwrap();

    // circle
    Circle::new(Point::new(88, yoffset), 16)
        .into_styled(style)
        .draw(display)
        .unwrap();

}

pub fn draw_some_text<D: DrawTarget<Color = BinaryColor>>(display: &mut D) where <D as DrawTarget>::Error: std::fmt::Debug
{
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X12)
        .text_color(BinaryColor::On)
        .build();

    Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
        .draw(display)
        .unwrap();

    Text::with_baseline("Hello Rust!", Point::new(0, 16), text_style, Baseline::Top)
        .draw(display)
        .unwrap();
}

