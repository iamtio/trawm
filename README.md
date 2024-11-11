# TRAWM - T Rust Air Wireless Monitoring
trawm is the firmware for [Pimoroni Badger 2040 W](https://shop.pimoroni.com/products/badger-2040-w) for air quality monitoring using [Airthings Wave Plus](https://www.airthings.com/wave-plus) via Bluetooth LE.
It doesn't require any additional settings, just install & run.

It's supposed to be energy efficient and work on AA/AAA batteries for months/years

# License
trawm is licensed under Apache License, Version 2.0 ([LICENSE](LICENSE) or <http://www.apache.org/licenses/LICENSE-2.0>)

# How it works

```mermaid
sequenceDiagram
    participant B as Badger 2040 W
    participant A as Airthings Wave Plus
    loop Every N minutes
        Note over B: Wake up by RTC alarm
        Note over B: BLE Scan
        opt Device found
            B->>A: BLE Connect
            A->>B: Get air quality values
            Note over B: Display values on E-Ink
        end
        Note over B: Set RTC alarm and deep sleep
    end
```
# How it looks
![trawm](https://github.com/user-attachments/assets/9436c888-21c7-4770-ac02-c87219a7a54f)

# How to flash the Badger 2040 W
- Clone this repo
- Install **rust** + **cargo** using **rustup**: <https://rustup.rs>
- Install **elf2uf2-rs**: `cargo install elf2uf2-rs`
- Connect Badger 2040 W
- Switch it to the boot-loader mode (Hold **reset** + **bootsel** buttons together, the **RPI-RP2** virtual disc should appear)
- Run `cargo build --release && cargo uf2-deploy` in project dir