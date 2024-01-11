// use crate::preludes::*;

use std::marker::PhantomData;

use embedded_graphics::{
    mono_font::MonoTextStyle,
    pixelcolor::PixelColor,
    text::Text, geometry::Point,
};

use embedded_layout::{
    layout::linear::{spacing, LinearLayout},
    prelude::*,
    ViewGroup,
};
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
    pub fn new(lbl: &'label str, style: MonoTextStyle<'static, C>) -> Self {
        Self {
            label: SimpleText::new(lbl, Point::zero(), style),
            text: SimpleText::new("", Point::zero(), style),
            style,
            _phantom: PhantomData,
        }
    }
}

impl<'label, C> LabeledTextBuilder<'label, C>
where
    C: PixelColor,
{
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
        // LabeledText {
        //     label: self.label,
        //     text: self.text,
        // }
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
    }
}


#[derive(Clone, Copy, Debug, ViewGroup)]
pub struct InputStatsRow<'label, C: PixelColor>
{
    button: LabeledText<'label, C>,
    motion: LabeledText<'label, C>,
}

impl<'label, C: PixelColor> InputStatsRow<'label, C> {
    pub fn new(style: MonoTextStyle<'static, C>) -> Self {
        let button = LabeledTextBuilder::new("Btn:", style).with_text("None").build();
        let motion = LabeledTextBuilder::new("Motion:", style).with_text("None").build();
        let row = Self {
            button,
            motion
        };
        LinearLayout::horizontal(row).with_spacing(spacing::DistributeFill(128)).arrange().into_inner()
    }

    pub fn set_button_text(&mut self, text: &'label str) {
        self.button.set_text(text);
    }

    pub fn set_motion_text(&mut self, text: &'label str) {
        self.motion.set_text(text);
    }

}
