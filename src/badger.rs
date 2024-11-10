use core::ops::Add;
use core::time::Duration;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use embassy_rp::i2c::I2c;
use embassy_rp::i2c::InterruptHandler as I2CInterruptHandler;
use embassy_rp::peripherals::*;
use embassy_rp::spi::Spi;
use embassy_rp::{i2c, spi};
use embassy_time::Delay;
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use pcf85063a::{Control, Error as PCFError, PCF85063};
use time::PrimitiveDateTime;
use uc8151::asynch::Uc8151;

embassy_rp::bind_interrupts!(struct Irqs {
    I2C0_IRQ => I2CInterruptHandler<I2C0>;
});

#[allow(non_snake_case)]
pub struct Badger2040wParams {
    pub I2C0: I2C0,
    pub PIN_5: PIN_5,
    pub PIN_4: PIN_4,
    pub PIN_17: PIN_17,
    pub PIN_20: PIN_20,
    pub PIN_26: PIN_26,
    pub PIN_21: PIN_21,
    pub SPI0: SPI0,
    pub PIN_18: PIN_18,
    pub PIN_19: PIN_19,
    pub PIN_16: PIN_16,
    pub DMA_CH1: DMA_CH1,
    pub DMA_CH2: DMA_CH2,
    pub PIN_10: PIN_10,
    pub PIN_22: PIN_22,
    pub PIN_15: PIN_15,
    pub PIN_11: PIN_11,
    pub PIN_12: PIN_12,
    pub PIN_13: PIN_13,
    pub PIN_14: PIN_14,
}

pub struct Badger2040wIO<'a> {
    pub power: Output<'a>,
    pub led: Output<'a>,
    pub btn_up: Input<'a>,
    pub btn_down: Input<'a>,
    pub btn_a: Input<'a>,
    pub btn_b: Input<'a>,
    pub btn_c: Input<'a>,
    pub display: Uc8151<
        ExclusiveDevice<Spi<'a, SPI0, spi::Async>, Output<'a>, NoDelay>,
        Output<'a>,
        Input<'a>,
        Output<'a>,
        Delay,
    >,
    rtc: PCF85063<I2c<'a, I2C0, i2c::Async>>,
}

impl<'a> Badger2040wIO<'a> {
    pub async fn init(p: Badger2040wParams) -> Badger2040wIO<'a> {
        // I2C for RTC
        let i2c = I2c::new_async(p.I2C0, p.PIN_5, p.PIN_4, Irqs, i2c::Config::default());

        // SPI for display
        let cs = Output::new(p.PIN_17, Level::High);
        let dc = Output::new(p.PIN_20, Level::Low);
        let busy = Input::new(p.PIN_26, Pull::Up);
        let reset = Output::new(p.PIN_21, Level::Low);
        let spi = Spi::new(
            p.SPI0,
            p.PIN_18,
            p.PIN_19,
            p.PIN_16,
            p.DMA_CH1,
            p.DMA_CH2,
            spi::Config::default(),
        );
        let spi_dev = ExclusiveDevice::new_no_delay(spi, cs);
        let mut rtc = PCF85063::new(i2c);
        rtc.clear_alarm_flag().await.unwrap();
        Badger2040wIO {
            power: Output::new(p.PIN_10, Level::Low),
            led: Output::new(p.PIN_22, Level::Low),
            btn_up: Input::new(p.PIN_15, Pull::Down),
            btn_down: Input::new(p.PIN_11, Pull::Down),
            btn_a: Input::new(p.PIN_12, Pull::Down),
            btn_b: Input::new(p.PIN_13, Pull::Down),
            btn_c: Input::new(p.PIN_14, Pull::Down),
            display: Uc8151::new(spi_dev, dc, busy, reset, Delay),
            rtc,
        }
    }
    pub async fn wake_up_in(
        self: &mut Self,
        duration: Duration,
    ) -> Result<(), PCFError<i2c::Error>> {
        self.rtc.clear_alarm_flag().await?;
        self.rtc
            .set_datetime(&PrimitiveDateTime::MIN)
            .await
            .unwrap();
        self.rtc
            .set_alarm_time(PrimitiveDateTime::MIN.time().add(duration))
            .await?;
        self.rtc.control_alarm_seconds(Control::On).await?;
        self.rtc.control_alarm_minutes(Control::On).await?;
        self.rtc.control_alarm_interrupt(Control::On).await?;
        Ok(())
    }
}
