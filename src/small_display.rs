



use esp_idf_hal::{gpio::{InputPin, OutputPin}, i2c::I2c};

use esp_idf_svc::{
    hal::{
        i2c::{I2cConfig, I2cDriver},
        peripheral::Peripheral,
        prelude::*
    },
};






use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    mono_font::{ascii::FONT_6X12, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
    primitives::{Circle, PrimitiveStyleBuilder, Rectangle, Triangle},
};

// type DisplayType<T> = impl DrawTarget<Color = T>;
// type SmallDisplay<'d> = Ssd1306<I2CInterface<I2cDriver<'d>>, DisplaySize128x64, ssd1306::mode::BasicMode>;
// type SmallDisplay<'d, SIZE, MODE> where SIZE: DisplaySize = Ssd1306<I2CInterface<I2cDriver<'d>>, SIZE, MODE>;
type SmallDisplay<'d, SIZE, MODE> = Ssd1306<I2CInterface<I2cDriver<'d>>, SIZE, MODE>;

pub fn init_display<'d>(
    i2c: impl Peripheral<P = impl I2c> + 'd,
    sda: impl Peripheral<P = impl InputPin + esp_idf_hal::gpio::OutputPin> + 'd,
    scl: impl Peripheral<P = impl InputPin + OutputPin> + 'd,
    size: impl DisplaySize,
    rotation: DisplayRotation,
) -> anyhow::Result<Ssd1306<I2CInterface<I2cDriver<'d>>, impl DisplaySize, ssd1306::mode::BasicMode>> {
    let config: esp_idf_hal::i2c::config::Config = I2cConfig::new().baudrate(400.kHz().into());
    let i2c: I2cDriver<'d> = I2cDriver::<'d>::new(i2c, sda, scl, &config)?;
    let interface: I2CInterface<I2cDriver<'d>> = I2CDisplayInterface::new(i2c);
    let disp = Ssd1306::new(interface, size, rotation);
    Ok(disp)
}

pub fn draw_shapes<D: DrawTarget<Color = BinaryColor>>(display: &mut D) where <D as DrawTarget>::Error: std::fmt::Debug
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

