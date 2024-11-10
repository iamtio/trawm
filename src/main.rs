#![no_std]
#![no_main]

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
use trawm::badger::*;
use trawm::ble::*;
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
    badger.display.setup(LUT::Internal).await.unwrap();
    let ble = BLE {
        PIN_25,
        PIO0,
        PIN_24,
        DMA_CH0,
        PIN_29,
        PIN_23,
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

    let mut awake_in = time::Duration::from_secs(90);
    let mut text: String<256> = String::new();
    // Try to get air metrics with 10 sec timout
    match ble.get_metrics(&spawner, Duration::from_secs(10)).await {
        Ok(metrics) => {
            write!(text, "{}", metrics).unwrap();
        }
        Err(e) => {
            write!(text, "An error occurred:\n{:?}", e).unwrap();
            awake_in = time::Duration::from_secs(10);
        }
    };

    let text_box = TextBox::with_textbox_style(&text, bounds, character_style, textbox_style);

    // Draw the text box.
    text_box.draw(&mut badger.display).unwrap();

    let _ = badger.display.update().await;

    defmt::info!("Going to deep sleep for {:?}", awake_in);
    badger.wake_up_in(awake_in).await.unwrap();
    badger.power.set_low();
    loop {
        // Will only run on USB power.
        log::info!("loop");
        Timer::after(Duration::from_secs(1)).await;
        badger.led.toggle();
    }
}
