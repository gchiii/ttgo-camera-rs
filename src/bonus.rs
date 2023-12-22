use anyhow::{anyhow, Error, bail, Result};
use std::{
    io::Cursor,
    time::Instant,
    sync::{Arc, Mutex},
};

use esp_idf_hal::reset::{ResetReason, WakeupReason};
use esp_idf_svc::{
    hal::{
        i2c::{I2cConfig, I2cDriver},
        peripherals::Peripherals,
        peripheral::Peripheral,
        timer::{Timer, TimerDriver},
        prelude::*
    },
    io::Write,
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    wifi::EspWifi,
    http::server::{Configuration, EspHttpServer},
};
use log::*;

use image::imageops::FilterType;
use image::io::Reader as ImageReader;
use image::{DynamicImage, GenericImageView};

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
use ttgo_camera::Camera;

fn image_to_ascii(image: DynamicImage) {
    let resolution = 8;
    let pallete: [char; 7] = [' ', '.', '/', '*', '#', '$', '@'];

    let mut y = 0;
    let small_img = image.resize_exact(
        image.width() / (resolution / 2),
        image.height() / resolution,
        FilterType::Nearest,
    );

    for p in small_img.pixels() {
        if y != p.1 {
            println!();
            y = p.1;
        }

        let r = p.2 .0[0] as f32;
        let g = p.2 .0[1] as f32;
        let b = p.2 .0[2] as f32;

        let k = r * 0.3 + g * 0.59 + b * 0.11;
        let character = ((k / 255.0) * (pallete.len() - 1) as f32).round() as usize;
        print!("{}", pallete[character]);
    }
}


fn draw_shapes<D: DrawTarget<Color = BinaryColor>>(display: &mut D) where <D as DrawTarget>::Error: std::fmt::Debug
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

fn draw_some_text<D: DrawTarget<Color = BinaryColor>>(display: &mut D) where <D as DrawTarget>::Error: std::fmt::Debug
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

fn take_pic(camera: &Camera<'_>) -> Result<DynamicImage, Error> {
    if let Some(fb) = camera.get_framebuffer() {
        let pic = fb.data_as_bmp()?;
        let image = ImageReader::new(Cursor::new(pic))
            .with_guessed_format()?
            .decode()?;
        Ok(image)
    } else {
        Err(anyhow!("Failed to get framebuffer"))
    }
}
