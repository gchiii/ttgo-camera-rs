// use crate::preludes::*;

use std::marker::PhantomData;

use embedded_graphics::{
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::FONT_6X12},
    pixelcolor::{PixelColor, BinaryColor},
    text::Text, geometry::Point,
};

use embedded_hal::digital;
use embedded_layout::{
    layout::linear::{spacing, LinearLayout},
    prelude::*,
    ViewGroup,
};
use log::info;
use once_cell::sync::Lazy;
use parking_lot::Mutex;

use crate::screen::StatusInfoGpio;

// use embedded_layout_macros::ViewGroup;



pub type SimpleStyle<C> = MonoTextStyle<'static, C>;
pub type SimpleText<'txt, C> = Text<'txt, SimpleStyle<C>>;


#[derive(Clone, Copy, Debug)]
pub struct LabeledTextBuilder<'label, C>
where
    C: PixelColor,
{
    label: SimpleText<'label, C>,
    text: SimpleText<'label, C>,
    style: SimpleStyle<C>,
    _phantom: PhantomData<&'label bool>,
}

impl<'label, C> LabeledTextBuilder<'label, C>
where
    C: PixelColor,
{
    pub const fn new(lbl: &'label str, style: MonoTextStyle<'static, C>) -> Self {
        Self {
            label: SimpleText::new(lbl, Point::zero(), style),
            text: SimpleText::new("", Point::zero(), style),
            style,
            _phantom: PhantomData,
        }
    }

    pub fn with_text(self, txt: &'label str) -> Self {
        Self {
            label: self.label,
            text: SimpleText::new(txt, Point::zero(), self.style)
             .align_to(&self.label, horizontal::LeftToRight, vertical::Top),
            style: self.style,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub fn build(self) -> LabeledText<'label, C> {
        LabeledText::new(self.label, self.text)
    }
}


#[derive(Clone, Copy, Debug, ViewGroup)]
pub struct LabeledText<'label, C: PixelColor>
{
    label: SimpleText<'label, C>,
    text: SimpleText<'label, C>,
}

impl<'label, C: PixelColor> LabeledText<'label, C> {
    pub fn new(label: SimpleText<'label, C>, text: SimpleText<'label, C>) -> Self {
        let lt =  Self {
            label,
            text
        };
        LinearLayout::horizontal(lt).with_spacing(spacing::FixedMargin(4)).arrange().into_inner()
    }

    pub fn set_text(&mut self, text: &'label str) {
        self.text.text = text;
        info!("LabeledText:set_text - {}", self.text.text);
    }

    pub fn with_text(&self, text: &'label str) -> Self {
        Self {
            label: self.label,
            text: SimpleText {
                text,
                ..self.text.clone()
            },
        }
    }
}


#[derive(Clone, Copy, Debug, ViewGroup)]
pub struct InputStatsRow<'label, C: PixelColor>
{
    button: LabeledText<'label, C>,
    motion: LabeledText<'label, C>,
}

static DEFAULT_TEXT_STYLE: Lazy<Mutex<MonoTextStyle<'static, BinaryColor>>> = Lazy::new(|| {
    Mutex::new(MonoTextStyleBuilder::new()
        .font(&FONT_6X12)
        .text_color(BinaryColor::On)
        .background_color(BinaryColor::Off)
        .build()
    )
});

impl<'label> Default for InputStatsRow<'label, BinaryColor> {
    fn default() -> Self {
        let style = *DEFAULT_TEXT_STYLE.lock();
        let button = LabeledTextBuilder::new("Btn:", style).with_text("None").build();
        let motion = LabeledTextBuilder::new("Motion:", style).with_text("None").build();
        let row = Self {
            button,
            motion
        };
        LinearLayout::horizontal(row).with_spacing(spacing::DistributeFill(128)).arrange().into_inner()
    }
}

pub fn gpio_status_as_str(gpio_stat: &StatusInfoGpio) -> &str {
    match gpio_stat {
        Some(digital::PinState::High) => "High",
        Some(digital::PinState::Low) => "Low",
        None => "None",
    }
}

impl<'label, C: PixelColor> InputStatsRow<'label, C> {
    pub fn new(style: MonoTextStyle<'static, C>, button: &'label StatusInfoGpio, motion: &'label StatusInfoGpio) -> Self {
        let button = LabeledTextBuilder::new("Btn:", style).with_text(gpio_status_as_str(button)).build();
        let motion = LabeledTextBuilder::new("Motion:", style).with_text(gpio_status_as_str(motion)).build();
        let row = Self {
            button,
            motion
        };
        LinearLayout::horizontal(row).with_spacing(spacing::DistributeFill(128)).arrange().into_inner()
    }

    pub fn do_arrange(&self) -> Self {
        LinearLayout::horizontal(*self).with_spacing(spacing::DistributeFill(128)).arrange().into_inner()
    }

    pub fn update_button(&self, button: &'label StatusInfoGpio) -> Self {
        Self {
            button: self.button.with_text(gpio_status_as_str(button)),
            motion: self.motion,
        }
    }

    pub fn update_motion(&self, motion: &'label StatusInfoGpio) -> Self {
        Self {
            button: self.button,
            motion: self.motion.with_text(gpio_status_as_str(motion)),
        }
    }
}
