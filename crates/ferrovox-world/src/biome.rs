//! # Biome system
//!
//! Biomes are determined from climate axes (temperature, humidity, continentalness)
//! and decide which surface block, height offset, and cave density applies.
//! The same biome table is used for both Java and Bedrock clients.

/// A biome definition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Biome {
    pub id: u8,
    pub name: &'static str,
    pub temperature: f32,
    pub humidity: f32,
    pub continentalness: f32,
    /// Surface block ID (grass, sand, etc.)
    pub surface_block: u16,
    /// Height offset added to base terrain height.
    pub height_offset: i32,
    /// Whether this biome is an ocean biome.
    pub is_ocean: bool,
    /// Whether trees generate here.
    pub has_trees: bool,
}

impl Default for Biome {
    fn default() -> Self {
        PLAINS
    }
}

/// All biome definitions — shared between Java and Bedrock.
pub mod biomes {
    use super::*;

    pub const OCEAN: Biome = Biome {
        id: 0,
        name: "ocean",
        temperature: 0.5,
        humidity: 0.5,
        continentalness: -0.4,
        surface_block: crate::blocks::SAND,
        height_offset: -10,
        is_ocean: true,
        has_trees: false,
    };

    pub const PLAINS: Biome = Biome {
        id: 1,
        name: "plains",
        temperature: 0.8,
        humidity: 0.4,
        continentalness: 0.1,
        surface_block: crate::blocks::GRASS_BLOCK,
        height_offset: 0,
        is_ocean: false,
        has_trees: true,
    };

    pub const FOREST: Biome = Biome {
        id: 2,
        name: "forest",
        temperature: 0.7,
        humidity: 0.8,
        continentalness: 0.2,
        surface_block: crate::blocks::GRASS_BLOCK,
        height_offset: 5,
        is_ocean: false,
        has_trees: true,
    };

    pub const DESERT: Biome = Biome {
        id: 3,
        name: "desert",
        temperature: 1.2,
        humidity: 0.1,
        continentalness: 0.3,
        surface_block: crate::blocks::SAND,
        height_offset: -2,
        is_ocean: false,
        has_trees: false,
    };

    pub const TAIGA: Biome = Biome {
        id: 4,
        name: "taiga",
        temperature: 0.2,
        humidity: 0.5,
        continentalness: 0.0,
        surface_block: crate::blocks::GRASS_BLOCK,
        height_offset: 3,
        is_ocean: false,
        has_trees: true,
    };

    pub const SNOWY_TUNDRA: Biome = Biome {
        id: 5,
        name: "snowy_tundra",
        temperature: -0.5,
        humidity: 0.3,
        continentalness: -0.1,
        surface_block: crate::blocks::GRASS_BLOCK,
        height_offset: 2,
        is_ocean: false,
        has_trees: false,
    };

    pub const JUNGLE: Biome = Biome {
        id: 6,
        name: "jungle",
        temperature: 1.1,
        humidity: 1.0,
        continentalness: 0.3,
        surface_block: crate::blocks::GRASS_BLOCK,
        height_offset: 8,
        is_ocean: false,
        has_trees: true,
    };

    pub const SAVANNA: Biome = Biome {
        id: 7,
        name: "savanna",
        temperature: 1.0,
        humidity: 0.2,
        continentalness: 0.2,
        surface_block: crate::blocks::GRASS_BLOCK,
        height_offset: 1,
        is_ocean: false,
        has_trees: true,
    };

    pub const MOUNTAINS: Biome = Biome {
        id: 8,
        name: "mountains",
        temperature: 0.3,
        humidity: 0.3,
        continentalness: 0.8,
        surface_block: crate::blocks::STONE,
        height_offset: 40,
        is_ocean: false,
        has_trees: true,
    };
}

use biomes::*;

// Public re-export of commonly used biomes
pub use biomes::*;

/// Select a biome from climate axes.
pub fn biome_at(temperature: f32, humidity: f32, continentalness: f32) -> Biome {
    // Ocean check first — low continentalness means ocean
    if continentalness < -0.2 {
        return OCEAN;
    }

    // Mountains — high continentalness
    if continentalness > 0.5 {
        return MOUNTAINS;
    }

    // Temperature + humidity classification
    if temperature > 1.0 {
        if humidity > 0.8 {
            JUNGLE
        } else if humidity < 0.3 {
            DESERT
        } else {
            SAVANNA
        }
    } else if temperature > 0.4 {
        if humidity > 0.7 {
            FOREST
        } else {
            PLAINS
        }
    } else if temperature > -0.2 {
        TAIGA
    } else {
        SNOWY_TUNDRA
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocean_biome() {
        let b = biome_at(0.5, 0.5, -0.5);
        assert_eq!(b.name, "ocean");
        assert!(b.is_ocean);
    }

    #[test]
    fn desert_biome() {
        let b = biome_at(1.2, 0.1, 0.3);
        assert_eq!(b.name, "desert");
    }

    #[test]
    fn jungle_biome() {
        let b = biome_at(1.1, 1.0, 0.3);
        assert_eq!(b.name, "jungle");
    }

    #[test]
    fn mountains_biome() {
        let b = biome_at(0.3, 0.3, 0.8);
        assert_eq!(b.name, "mountains");
    }

    #[test]
    fn plains_default() {
        let b = biome_at(0.8, 0.4, 0.1);
        assert_eq!(b.name, "plains");
    }
}
