use bt_hci::controller::ExternalController;
use bt_hci::param::LeAdvEventKind::AdvInd;
use core::iter;
use cyw43_pio::PioSpi;
use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::*;
use embassy_rp::pio::{InterruptHandler as PIOInterruptHandler, Pio};
use embassy_time::{with_timeout, Duration as EmbassyDuration};

use heapless::Vec;
use static_cell::StaticCell;
use trouble_host::advertise::AdStructure;
use trouble_host::gatt::GattClient;
use trouble_host::prelude::{ConnectConfig, Uuid};
use trouble_host::scan::ScanConfig;
use trouble_host::{Address, HostResources, PacketQos};

use crate::metrics::{AirMetrics, ParseMetricsError};

type BleResources<C> = HostResources<C, 1, 3, 27>;

embassy_rp::bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PIOInterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}
// b42e1c08-ade7-11e4-89d3-123b93f75cba
const SERVICE_UUID: Uuid = Uuid::new_long([
    0xba, 0x5c, 0xf7, 0x93, 0x3b, 0x12, 0xd3, 0x89, 0xe4, 0x11, 0xe7, 0xad, 0x08, 0x1c, 0x2e, 0xb4,
]);

// b42e2a68-ade7-11e4-89d3-123b93f75cba
const CHAR_UUID: Uuid = Uuid::new_long([
    0xba, 0x5c, 0xf7, 0x93, 0x3b, 0x12, 0xd3, 0x89, 0xe4, 0x11, 0xe7, 0xad, 0x68, 0x2a, 0x2e, 0xb4,
]);

#[derive(Debug, Clone, Copy)]
pub enum BLEError {
    ConnectionProblem,
    ServiceNotFound,
    CharacteristicsNotFound,
    ParseMetricsProblem(ParseMetricsError),
    TimedOut,
}

#[allow(non_snake_case)]
pub struct BLE {
    pub PIN_25: PIN_25,
    pub PIO0: PIO0,
    pub PIN_24: PIN_24,
    pub DMA_CH0: DMA_CH0,
    pub PIN_29: PIN_29,
    pub PIN_23: PIN_23,
}
impl BLE {
    pub async fn get_metrics(
        self: Self,
        spawner: &Spawner,
        operation_timeout: EmbassyDuration,
    ) -> Result<AirMetrics, BLEError> {
        // Loading wireless firmware
        let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
        let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");
        let btfw = include_bytes!("../cyw43-firmware/43439A0_btfw.bin");

        // IO
        let pwr = Output::new(self.PIN_23, Level::Low);
        let cs = Output::new(self.PIN_25, Level::High);
        let mut pio = Pio::new(self.PIO0, Irqs);
        let spi = PioSpi::new(
            &mut pio.common,
            pio.sm0,
            pio.irq0,
            cs,
            self.PIN_24,
            self.PIN_29,
            self.DMA_CH0,
        );
        // BLE
        static STATE: StaticCell<cyw43::State> = StaticCell::new();
        let state = STATE.init(cyw43::State::new());
        let (_net_device, bt_device, mut control, runner) =
            cyw43::new_with_bluetooth(state, pwr, spi, fw, btfw).await;
        spawner.spawn(cyw43_task(runner)).unwrap();
        control.init(clm).await;

        let controller: ExternalController<_, 10> = ExternalController::new(bt_device);
        let mut resources = BleResources::new(PacketQos::None);
        let (stack, _, mut central, mut trouble_runner) =
            trouble_host::new(controller, &mut resources).build();

        // Results
        let mut found_addr: Option<Address> = None;
        let mut raw_metrics = [0; 256];
        let mut scan_and_fetch_error: Option<BLEError> = None;
        let scan_and_fetch = async {
            defmt::info!("Scan start");

            'scan: loop {
                let reports = central.scan(&ScanConfig::default()).await;
                let Ok(reports) = reports else {
                    defmt::error!("BLEHostError");
                    continue;
                };
                for report in reports.iter() {
                    let Ok(report) = report else {
                        defmt::error!("FromHCIBytesError");
                        continue;
                    };
                    if report.event_kind != AdvInd {
                        // https://academy.nordicsemi.com/courses/bluetooth-low-energy-fundamentals/lessons/lesson-2-bluetooth-le-advertising/topic/advertising-types/
                        continue;
                    }
                    defmt::info!(
                        "> {:?}\t{:X}: {:X}",
                        report.event_kind,
                        report.addr,
                        report.data
                    );
                    let mut fixed_report_data = Vec::<u8, 256>::new();
                    fix_adv_payload(&report.data, &mut fixed_report_data);
                    defmt::info!(
                        "= {:?}\t{:X}: {:X}",
                        report.event_kind,
                        report.addr,
                        fixed_report_data
                    );

                    for ad in AdStructure::decode(&fixed_report_data[..]) {
                        let ad = match ad {
                            Ok(ad) => ad,
                            Err(e) => {
                                defmt::error!("Structure decode error: {:?}", e);
                                break;
                            }
                        };
                        defmt::info!("{:?}", ad);
                        match ad {
                            AdStructure::ManufacturerSpecificData {
                                company_identifier: 0x0334,
                                ..
                            } => {
                                found_addr = Some(Address {
                                    kind: report.addr_kind,
                                    addr: report.addr,
                                });
                                break 'scan;
                            }
                            _ => (),
                        }
                    }
                }
            }
            // Fetch metrics
            defmt::info!("Found airthings. {:?}", found_addr);
            let Some(target) = found_addr else {
                defmt::error!("Couldn't connect. No address specified");
                scan_and_fetch_error = Some(BLEError::ConnectionProblem);
                return;
            };
            let conn = central
                .connect(&ConnectConfig {
                    connect_params: Default::default(),
                    scan_config: ScanConfig {
                        filter_accept_list: &[(target.kind, &target.addr)],
                        ..Default::default()
                    },
                })
                .await;
            let Ok(conn) = conn else {
                scan_and_fetch_error = Some(BLEError::ConnectionProblem);
                return;
            };
            defmt::info!("Connected, creating gatt client");

            let Ok(client) = GattClient::<_, 10, 27>::new(stack, &conn).await else {
                scan_and_fetch_error = Some(BLEError::ConnectionProblem);
                return;
            };

            let _ = select(client.task(), async {
                defmt::info!("Looking for Airthings metrics service");
                let Ok(services) = client.services_by_uuid(&SERVICE_UUID).await else {
                    scan_and_fetch_error = Some(BLEError::ServiceNotFound);
                    return;
                };
                let Some(service) = services.first() else {
                    scan_and_fetch_error = Some(BLEError::ServiceNotFound);
                    return;
                };

                defmt::info!("Looking for Airthings metrics characteristics");
                let characteristic = client
                    .characteristic_by_uuid(&service.clone(), &CHAR_UUID)
                    .await;
                let Ok(characteristic) = characteristic else {
                    scan_and_fetch_error = Some(BLEError::CharacteristicsNotFound);
                    return;
                };
                let _ = client
                    .read_characteristic(&characteristic, &mut raw_metrics[..])
                    .await;
                defmt::info!("Got characteristics: {:X}", raw_metrics);
            })
            .await;
        };
        let mut timeout_error: Option<BLEError> = None;
        select(trouble_runner.run(), async {
            let Ok(_) = with_timeout(operation_timeout, scan_and_fetch).await else {
                defmt::error!("Scan timed out");
                timeout_error = Some(BLEError::TimedOut);
                return;
            };
        })
        .await;
        match scan_and_fetch_error {
            Some(e) => Err(e),
            None => match timeout_error {
                Some(e) => Err(e),
                None => match AirMetrics::from_bytes(&raw_metrics) {
                    Ok(result) => Ok(result),
                    Err(err) => Err(BLEError::ParseMetricsProblem(err)),
                },
            },
        }
    }
}

fn fix_adv_payload<const N: usize>(payload: &[u8], result: &mut Vec<u8, N>) {
    if payload.is_empty() {
        return;
    }
    let mut pos: usize = 0;
    loop {
        let chunk_len = payload[pos] as usize;
        result.push(payload[pos]).unwrap();
        pos += 1;
        let bytes_left = payload[pos..].len();
        defmt::debug!(
            "pos: {}, chunk_len: {}, bytes_left: {}",
            pos,
            chunk_len,
            bytes_left
        );

        let offset = if bytes_left < chunk_len {
            bytes_left
        } else {
            chunk_len
        };
        result.extend(payload[pos..pos + offset].iter().cloned());
        if bytes_left < chunk_len {
            // Probably at the end
            result.extend(iter::repeat(0u8).take(chunk_len - bytes_left));
            return;
        }
        pos += chunk_len;
        if pos >= payload.len() {
            return;
        }
    }
}
