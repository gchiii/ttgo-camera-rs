use std::marker::PhantomData;
use std::net::Ipv4Addr;
use std::thread;
use std::time::Duration;

use embedded_graphics::geometry::AnchorPoint;
use embedded_graphics::primitives::*;

use embedded_layout::{
    layout::linear::{spacing, Horizontal, LinearLayout, Vertical},
    prelude::*,
    ViewGroup,
};

use embedded_layout::align::*;

use embedded_layout::object_chain::Chain;


use embedded_text::TextBox;
use embedded_text::alignment::HorizontalAlignment;
use embedded_text::style::{TextBoxStyleBuilder, HeightMode};
use esp_idf_hal::gpio::{InputPin, OutputPin, self};
use esp_idf_hal::i2c::{I2c, I2cDriver, I2cConfig};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::units::*;

use esp_idf_sys::EspError;
use log::info;
use log::error;
use once_cell::sync::{OnceCell, Lazy};
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

use crate::small_display;

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

type DefaultTextStyle<'a> = MonoTextStyle<'a, BinaryColor>;
type MsgBoxText<'a> = Text<'a, DefaultTextStyle<'a>>;
#[derive(Clone, Debug)]
pub struct MsgBox<'a> {
    char_style: DefaultTextStyle<'a>,
    bounds: Rectangle,
    _phantom: PhantomData<&'a bool>,
}

static DEFAULT_TEXT_STYLE: Lazy<Mutex<MonoTextStyle<'static, BinaryColor>>> = Lazy::new(|| {
    Mutex::new(MonoTextStyleBuilder::new()
        .font(&FONT_6X12)
        .text_color(BinaryColor::On)
        .build()
    )
});

impl<'a> Default for MsgBox<'a> {
    fn default() -> Self {
        let char_style = *DEFAULT_TEXT_STYLE.lock();
        // let char_style: MonoTextStyle<'a, BinaryColor> = MonoTextStyleBuilder::new()
        //     .font(&FONT_6X12)
        //     .text_color(BinaryColor::On)
        //     .build();
        Self { char_style, bounds: Default::default(), _phantom: PhantomData }
    }
}

impl<'a> MsgBox<'a> {
    pub fn new(bounds: Rectangle) -> Self {
        Self {
            bounds,
            ..Default::default()
        }
    }

    pub fn create(bounds_box: Rectangle, label: &'a str, text: &'a str) -> LinearLayout<Horizontal<vertical::Top>, Link<MsgBoxText<'a>, Chain<MsgBoxText<'a>>>>     {
        let char_style = *DEFAULT_TEXT_STYLE.lock();

        let line_one = LinearLayout::horizontal(
            Chain::new(
                Text::new(label, Point::zero(), char_style))
                .append(Text::new(text, Point::zero(), char_style))
        )
        .with_alignment(vertical::Top)
        .arrange()
        .align_to(&bounds_box, horizontal::Center, vertical::Center);

        line_one
    }


}

//  where <D as DrawTarget>::Error: std::fmt::Debug)
pub fn msg_box<D: DrawTarget<Color = BinaryColor>>(msg: &str, bounds_box: Rectangle, display: &mut D) -> Result<(),SmallDisplayError> {
    let character_style = MonoTextStyle::new(&FONT_6X12, BinaryColor::On);
    let textbox_style = TextBoxStyleBuilder::new()
        .height_mode(HeightMode::FitToText)
        .alignment(HorizontalAlignment::Justified)
        .paragraph_spacing(6)
        .build();
    let line_one = LinearLayout::horizontal(
        Chain::new(Text::new("IP:", Point::zero(), character_style))
            .append(Text::new(msg, Point::zero(), character_style))

    )
    .with_alignment(vertical::Top)
    .arrange()
    .align_to(&bounds_box, horizontal::Center, vertical::Center);

    // Specify the bounding box. Note the 0px height. The `FitToText` height mode will
    // measure and adjust the height of the text box in `into_styled()`.
    // Create the text box and apply styling options.
    // TextBox::with_textbox_style(msg, bounds_box, character_style, textbox_style)
    Ok(())
}


#[derive(Clone, Debug)]
pub enum InfoUpdate {
    Addr(Ipv4Addr),
    Button(gpio::Level),
    Motion(gpio::Level),
    Msg(String),
}


// screen width 128
// screen height 64
// top line is the ip address
// next line has PIR and Button
// use the rest for messages


pub fn layout_box<'a>(label: &'a str, text: &'a str) -> LinearLayout<Horizontal<embedded_layout::align::vertical::Bottom>, embedded_layout::object_chain::Link<embedded_graphics::text::Text<'a, MonoTextStyle<'a, BinaryColor>>, embedded_layout::object_chain::Chain<embedded_graphics::text::Text<'a, MonoTextStyle<'a, BinaryColor>>>>> {
    let char_style = *DEFAULT_TEXT_STYLE.lock();
    let mut lbl = Text::new(label, Point::zero(), char_style);
    let mut val = Text::new(text, Point::zero(), char_style);
    LinearLayout::horizontal(Chain::new(lbl).append(val)).arrange()
}

pub fn box_test<'d>(interface: I2CInterface<I2cDriver<'d>>) -> Result<(), SmallDisplayError> {
    info!("started display_runner!!!!!!!");
    let character_style = *DEFAULT_TEXT_STYLE.lock();
    let mut display = init_display(interface, DisplaySize128x64, DisplayRotation::Rotate0).unwrap()
        .into_buffered_graphics_mode();
    display.init()?;
    display.init()?;

    let m_box = small_display::MsgBox::create(display.bounding_box(), "Hello", "World");
    m_box.draw(&mut display)?;
    display.flush()?;

    Ok(())
}


#[derive(ViewGroup, Clone)]
struct LabeledText<'txt, C: PixelColor> {
    label: Text<'txt, MonoTextStyle<'static, C>>,
    text: Text<'txt, MonoTextStyle<'static, C>>,
}

impl<'txt, C: PixelColor> LabeledText<'txt, C> {
    fn new(lbl: &'txt str, txt: &'txt str, char_style: MonoTextStyle<'static, C>) -> Self {
        let label: Text<'txt, MonoTextStyle<'static, C>> = Text::new(lbl, Point::zero(), char_style);
        let text: Text<'txt, MonoTextStyle<'static, C>> = Text::new(txt, Point::zero(), char_style);
        Self { label, text }
    }
}

#[derive(Clone, ViewGroup)]
struct LayoutLabeledText<'txt, C: PixelColor> {
    layout: LinearLayout<Horizontal<vertical::Center, spacing::FixedMargin>, LabeledText<'txt, C>>,
}

impl<'txt, C: PixelColor> LayoutLabeledText<'txt, C> {
    fn new(lbl: &'txt str, txt: &'txt str, char_style: MonoTextStyle<'static, C>) -> Self {
        let labeled_text: LabeledText<'txt, C> = LabeledText::new(lbl, txt, char_style);
        // let layout: LinearLayout<Horizontal<vertical::Center, spacing::FixedMargin>, LabeledText<'txt, C>> =
        let layout = LinearLayout::horizontal(labeled_text)
            .with_alignment(vertical::Center)
            .with_spacing(spacing::FixedMargin(3));
        Self { layout }
    }
}

#[derive(ViewGroup, Clone)]
struct InputRow<'txt, C: PixelColor> {
    button: LayoutLabeledText<'txt, C>,
    motion: LayoutLabeledText<'txt, C>,
}

impl<'txt, C: PixelColor> InputRow<'txt, C> {
    fn new(button_text: &'txt str, motion_text: &'txt str, char_style: MonoTextStyle<'static, C>) -> Self {
        let button: LayoutLabeledText<'txt, C> = LayoutLabeledText::new("btn:", button_text, char_style);
        let motion: LayoutLabeledText<'txt, C> = LayoutLabeledText::new("pir:", motion_text, char_style);
        Self {
            button,
            motion
        }
    }
}

#[derive(ViewGroup, Clone)]
struct LayoutInputRow<'txt, C: PixelColor> {
    layout: LinearLayout<
        Horizontal<vertical::Center, spacing::FixedMargin>,
        InputRow<'txt, C>,
    >,
}

impl<'txt, C: PixelColor> LayoutInputRow<'txt, C> {
    fn new(button_text: &'txt str, motion_text: &'txt str, char_style: MonoTextStyle<'static, C>) -> Self {
        let input_row = InputRow::new(button_text, motion_text, char_style);
        let layout = LinearLayout::horizontal(input_row)
            .with_alignment(vertical::Center)
            .with_spacing(spacing::FixedMargin(3));
        Self { layout }
    }
}

#[derive(ViewGroup)]
struct LayoutStatusWindow<'txt, C: PixelColor> {
    layout: LinearLayout<
        Vertical<horizontal::Center, spacing::Tight>,
        chain! {
            LayoutLabeledText<'txt, C>,
            LayoutInputRow<'txt, C>
        }
    >,
}

impl<'txt, C: PixelColor> Clone for LayoutStatusWindow<'txt, C> {
    fn clone(&self) -> Self {
        Self { layout: self.layout.clone() }
    }
}

impl<'txt, C: PixelColor> LayoutStatusWindow<'txt, C> {
    fn new(ip: &'txt str, button_text: &'txt str, motion_text: &'txt str, char_style: MonoTextStyle<'static, C>) -> Self {
        let ip_row = LayoutLabeledText::new("IP:", ip, char_style);
        let inp_row = LayoutInputRow::new(button_text, motion_text, char_style);
        let layout = LinearLayout::vertical(Chain::new(ip_row).append(inp_row))
            .with_alignment(horizontal::Center)
            .with_spacing(spacing::Tight);
        Self { layout }
    }
}


const LONGEST_IPV4_ADDR: &str = "255.255.255.255";
#[derive(Clone)]
pub struct InfoScreen<'txt, C: PixelColor> {
    character_style: MonoTextStyle<'static, C>,
    ip_address: heapless::String<{LONGEST_IPV4_ADDR.len()}>,
    pir_str: heapless::String<5>,
    btn_str: heapless::String<5>,
    _phantom: PhantomData<&'txt bool>,
}

impl<'txt, C: PixelColor> InfoScreen<'txt, C> {
    pub fn new(ip: &Ipv4Addr, pir: &str, btn: &str, character_style: MonoTextStyle<'static, C>) -> Self {
        let pir_str = match heapless::String::<5>::try_from(pir) {
            Ok(s) => s,
            Err(_) => heapless::String::new(),
        };
        let btn_str = match heapless::String::<5>::try_from(btn) {
            Ok(s) => s,
            Err(_) => heapless::String::new(),
        };
        let tmp_ip = ip.to_string();
        let ip_address = match heapless::String::<{LONGEST_IPV4_ADDR.len()}>::try_from(tmp_ip.as_str()) {
            Ok(s) => s,
            Err(_) => heapless::String::new(),
        };
        Self {
            character_style,
            ip_address,
            pir_str,
            btn_str,
            _phantom: PhantomData,
        }
    }

    pub fn layout(&'txt self) -> LayoutStatusWindow<'txt, C> {
        LayoutStatusWindow::new(&self.ip_address, &self.btn_str, &self.pir_str, self.character_style)
    }

    pub fn pir(&'txt mut self, pir: &str) -> &mut InfoScreen<'_, C> {
        if let Ok(s) = heapless::String::<5>::try_from(pir) {
            self.pir_str = s;
        }
        self
    }

    pub fn btn(&'txt mut self, btn: &str) -> &mut InfoScreen<'_, C> {
        if let Ok(s) = heapless::String::<5>::try_from(btn) {
            self.btn_str = s;
        }
        self
    }

    pub fn addr(&'txt mut self, ip: &Ipv4Addr) -> &mut InfoScreen<'_, C> {
        if let Ok(s) = heapless::String::<{LONGEST_IPV4_ADDR.len()}>::try_from(ip.to_string().as_str()) {
            self.ip_address = s;
        }
        self
    }


}




pub async fn display_runner<'d>(interface: I2CInterface<I2cDriver<'d>>, rx: flume::Receiver<InfoUpdate>) -> Result<(), SmallDisplayError> {
    info!("started display_runner!!!!!!!");
    let character_style = *DEFAULT_TEXT_STYLE.lock();
    let mut display = init_display(interface, DisplaySize128x64, DisplayRotation::Rotate0).unwrap()
        .into_buffered_graphics_mode();
    display.init()?;
    display.init()?;

    display.flush()?;

    let display_bounds = display.bounding_box();

    let status_thing = InfoScreen::new(&Ipv4Addr::UNSPECIFIED, "text", "text", character_style);
    status_thing.layout().draw(&mut display)?;
    display.flush()?;
    thread::sleep(Duration::from_secs(2));
    display.clear_buffer();
    display.flush()?;

    // let ip_box = Rectangle::new(Point::zero(), Size::new(display_bounds.size.width, character_style.font.character_size.height));
    // let pz = Point::zero();
    let mut ip_str = Ipv4Addr::UNSPECIFIED.to_string();
    let mut btn_str = String::from("text");
    let mut pir_str = String::from("text");
    let mut messages = String::new();

    let mut ip_display = layout_box("IP: ", ip_str.as_str() );
    ip_display.draw(&mut display)?;
    display.flush()?;
    thread::sleep(Duration::from_secs(2));
    display.clear_buffer();
    display.flush()?;

    let mut btn_display = layout_box("btn: ", &btn_str);
    btn_display.draw(&mut display)?;
    display.flush()?;
    thread::sleep(Duration::from_secs(2));
    display.clear_buffer();
    display.flush()?;

    let mut pir_display = layout_box("pir: ", &pir_str);
    pir_display.draw(&mut display)?;
    display.flush()?;
    thread::sleep(Duration::from_secs(2));
    display.clear_buffer();
    display.flush()?;

    let mut inp_display = LinearLayout::horizontal(Chain::new(btn_display).append(pir_display)).arrange();
    inp_display.draw(&mut display)?;
    display.flush()?;
    thread::sleep(Duration::from_secs(2));
    display.clear_buffer();
    display.flush()?;

    let mut status_disp = LinearLayout::vertical(Chain::new(ip_display).append(inp_display))
        .arrange()
        .align_to(&display_bounds, horizontal::Center, vertical::Top);
    status_disp.draw(&mut display)?;
    display.flush()?;
    thread::sleep(Duration::from_secs(2));
    display.clear_buffer();
    display.flush()?;

    let textbox_style = TextBoxStyleBuilder::new()
        .height_mode(HeightMode::FitToText)
        .alignment(HorizontalAlignment::Justified)
        .paragraph_spacing(6)
        .build();
    let bounds = Rectangle::new(status_disp.bounds().anchor_point(AnchorPoint::BottomLeft), display_bounds.size - status_disp.size());

    let mut text_box = TextBox::with_textbox_style(messages.as_str(), bounds, character_style, textbox_style);
    text_box.draw(&mut display)?;

    let mut text_point = Point::zero();

    loop {
        let _ = flume::Selector::new()
            .recv(&rx, |thing| {
                let info_update = match thing {
                    Ok(x) => x,
                    Err(e) => {
                        error!("error: {:?}", e);
                        return Err(SmallDisplayError::Other(format!("{}", e)));
                    },
                };
                match info_update {
                    InfoUpdate::Addr(ip) => {
                        let ip_str = ip.to_string();
                        info!("ip: {}", ip_str);
                        // ip_display.inner().object.text = &ip_str;
                        status_disp.draw(&mut display)?;
                    },
                    InfoUpdate::Button(level) => {
                        let btn_str = format!("{:?}", level);
                        info!("btn: {}", btn_str);
                        status_disp.draw(&mut display)?;
                    },
                    InfoUpdate::Motion(level) => {
                        let pir_str = format!("{:?}", level);
                        info!("pir: {}", pir_str);
                        status_disp.draw(&mut display)?;
                    },
                    InfoUpdate::Msg(text) => {
                        info!("disp: {}", text);
                        // messages.push_str("\n");
                        // messages.push_str(&text);
                        // text_box.text = messages.as_str();
                        text_box.draw(&mut display)?;
                    },
                }
                display.flush()?;
                Ok(())
            })
            .wait();
    }
}
