use serde::{Deserialize, Serialize};
// 0.17.1
use strum_macros::{EnumIter, Display}; // 0.17.1

#[derive(EnumIter, Display, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemperatureUnit {
    Kelvin,
    Celsius,
    Fahrenheit,
}

impl Default for TemperatureUnit {
    fn default() -> Self {
        TemperatureUnit::Kelvin
    }
}

impl TemperatureUnit {
    pub fn suffix(&self) -> String {
        match self {
            TemperatureUnit::Kelvin => "K".to_string(),
            TemperatureUnit::Celsius => "°C".to_string(),
            TemperatureUnit::Fahrenheit => "°F".to_string(),
        }
    }

    pub fn from_kelvin(&self, kelvin: f32) -> f32 {
        match self {
            TemperatureUnit::Kelvin => kelvin,
            TemperatureUnit::Celsius => kelvin - 273.15,
            TemperatureUnit::Fahrenheit => (kelvin - 273.15) * 1.8 + 32.0,
        }
    }

    pub fn to_kelvin(&self, value: f32) -> f32 {
        match self {
            TemperatureUnit::Kelvin => value,
            TemperatureUnit::Celsius => value + 273.15,
            TemperatureUnit::Fahrenheit => (value - 32.0) / 1.8 + 273.15,
        }
    }
}
