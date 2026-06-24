// src/config.rs
use ratatui::style::Color;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Единый модуль для сериализации цвета
pub mod color_serde {
    use super::*;

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match color {
            Color::Cyan => "Cyan",
            Color::Green => "Green",
            Color::Magenta => "Magenta",
            Color::Yellow => "Yellow",
            Color::Red => "Red",
            Color::White => "White",
            _ => "Cyan",
        };
        s.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.as_str() {
            "Cyan" => Color::Cyan,
            "Green" => Color::Green,
            "Magenta" => Color::Magenta,
            "Yellow" => Color::Yellow,
            "Red" => Color::Red,
            "White" => Color::White,
            _ => Color::Cyan,
        })
    }
}
