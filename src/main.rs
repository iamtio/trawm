#![no_std]
#![no_main]

use trawm::badger::*;
use trawm::ble::*;
use core::fmt::Write;
use core::time;
use defmt;
use embassy_executor::Spawner;
use embassy_rp::Peripherals;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    mono_font::{ascii::*, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};
use embedded_text::{
    alignment::HorizontalAlignment,
    style::{HeightMode, TextBoxStyleBuilder},
    TextBox,
};
use heapless::String;
use uc8151::LUT;
use uc8151::WIDTH;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    defmt::info!("Starting");
    #[allow(non_snake_case)]
    let Peripherals {
        PIN_25,
        PIO0,
        PIN_24,
        PIN_29,
        PIN_23,
        I2C0,
        PIN_5,
        PIN_4,
        PIN_17,
        PIN_20,
        PIN_26,
        PIN_21,
        SPI0,
        PIN_18,
        PIN_19,
        PIN_16,
        DMA_CH0,
        DMA_CH1,
        DMA_CH2,
        PIN_10,
        PIN_22,
        PIN_15,
        PIN_11,
        PIN_12,
        PIN_13,
        PIN_14,
        ..
    } = embassy_rp::init(Default::default());
    let mut badger = Badger2040wIO::init(Badger2040wParams {
        I2C0,
        PIN_5,
        PIN_4,
        PIN_17,
        PIN_20,
        PIN_26,
        PIN_21,
        SPI0,
        PIN_18,
        PIN_19,
        PIN_16,
        DMA_CH1,
        DMA_CH2,
        PIN_10,
        PIN_22,
        PIN_15,
        PIN_11,
        PIN_12,
        PIN_13,
        PIN_14,
    })
    .await;
    badger.power.set_high();
    badger.led.set_high();
    badger.display.reset().await;
    // Initialise display. Using the default LUT speed setting
    let _ = badger.display.setup(LUT::Internal).await;
    let metrics = BLE {
        PIN_25,
        PIO0,
        PIN_24,
        DMA_CH0,
        PIN_29,
        PIN_23,
    }
    .get_metrics(&spawner, Duration::from_secs(10))
    .await;
    defmt::info!("Got metrics: {:?}", metrics);
    let Some(metrics) = metrics else {
        badger.power.set_low();
        return;
    };
    // Note we're setting the Text color to `Off`. The driver is set up to treat Off as Black so that BMPs work as expected.
    let character_style = MonoTextStyle::new(&FONT_9X18_BOLD, BinaryColor::Off);
    let textbox_style = TextBoxStyleBuilder::new()
        .height_mode(HeightMode::FitToText)
        .alignment(HorizontalAlignment::Left)
        .paragraph_spacing(0)
        .build();

    // Bounding box for our text. Fill it with the opposite color so we can read the text.
    let bounds = Rectangle::new(Point::new(0, 0), Size::new(WIDTH, 0));
    bounds
        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
        .draw(&mut badger.display)
        .unwrap();
    // Create the text box and apply styling options.
    let mut metrics_text: String<256> = String::new();
    write!(metrics_text, "{}", metrics).unwrap();
    let text_box =
        TextBox::with_textbox_style(&metrics_text, bounds, character_style, textbox_style);

    // Draw the text box.
    text_box.draw(&mut badger.display).unwrap();

    let _ = badger.display.update().await;

    badger
        .wake_up_in(time::Duration::from_secs(10))
        .await
        .unwrap();
    loop {
        badger.power.set_low();
        log::info!("loop");
        Timer::after(Duration::from_secs(1)).await;
        badger.led.toggle();
    }
}
