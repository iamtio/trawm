use core::fmt;

#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct AirMetrics {
    pub version: u8,
    pub humidity: f32,
    pub illuminance: f32,
    pub radon_short: u16,
    pub radon_long: u16,
    pub temperature: f32,
    pub pressure: f32,
    pub co2_level: u16,
    pub voc_level: u16,
}

#[derive(Debug, Clone, Copy)]
pub enum ParseMetricsError {
    InsufficientBytes,
    UnsupportedPacketVersion,
}

impl AirMetrics {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ParseMetricsError> {
        // struct format: <BBBxHHHHHHxxxx
        if bytes.len() < 16 {
            Err(ParseMetricsError::InsufficientBytes)
        } else if bytes[0] != 1 {
            Err(ParseMetricsError::UnsupportedPacketVersion)
        } else {
            Ok(Self {
                version: bytes[0],
                humidity: bytes[1] as f32 / 2.0,
                illuminance: bytes[2] as f32 / 255.0 * 100.0,
                radon_short: u16::from_le_bytes([bytes[4], bytes[5]]),
                radon_long: u16::from_le_bytes([bytes[6], bytes[7]]),
                temperature: u16::from_le_bytes([bytes[8], bytes[9]]) as f32 / 100.0,
                pressure: u16::from_le_bytes([bytes[10], bytes[11]]) as f32 / 50.0,
                co2_level: u16::from_le_bytes([bytes[12], bytes[13]]),
                voc_level: u16::from_le_bytes([bytes[14], bytes[15]]),
            })
        }
    }
}
impl fmt::Display for AirMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Humidity: {0:.2} %\n\
            Temperature: {1:.2} °C\n\
            Pressure: {2:.2} hPa\n\
            CO2: {3} ppm\n\
            VOC: {4} ppb\n\
            Radon 1day: {5} Long: {6}",
            self.humidity,
            self.temperature,
            self.pressure,
            self.co2_level,
            self.voc_level,
            self.radon_short,
            self.radon_long
        )
    }
}
