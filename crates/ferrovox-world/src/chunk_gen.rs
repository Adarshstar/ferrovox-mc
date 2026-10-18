//! # Chunk generation
//!
//! Fills a 16x16x384 block column from the WorldGen and exposes it as a
//! contiguous block array that both protocol front-ends can serialize.

use crate::blocks::*;
use crate::{WorldGen, SEA_LEVEL, WORLD_BOTTOM_Y, WORLD_HEIGHT};

/// A generated 16x16 chunk section (one Y-section of 16 blocks).
#[derive(Debug, Clone, Default)]
pub struct ChunkSection {
    /// 16x16x16 blocks, indexed as [y][z][x].
    pub blocks: [[[u16; 16]; 16]; 16],
    /// Whether this section is entirely air (for optimization).
    pub is_empty: bool,
}

/// A full chunk: 16x16x(WORLD_HEIGHT) blocks divided into sections.
#[derive(Debug, Clone, Default)]
pub struct Chunk {
    pub sections: Vec<ChunkSection>,
    pub chunk_x: i32,
    pub chunk_z: i32,
}

impl Chunk {
    /// Create an empty chunk with all sections initialized to air.
    pub fn new(chunk_x: i32, chunk_z: i32) -> Self {
        let section_count = (WORLD_HEIGHT / 16) as usize;
        let mut sections = Vec::with_capacity(section_count);
        for _ in 0..section_count {
            sections.push(ChunkSection::default());
        }
        Self { sections, chunk_x, chunk_z }
    }

    /// Get a mutable reference to a section by Y index (0 = bottom).
    pub fn section_mut(&mut self, y_index: usize) -> Option<&mut ChunkSection> {
        self.sections.get_mut(y_index)
    }

    /// Get a section by Y index.
    pub fn section(&self, y_index: usize) -> Option<&ChunkSection> {
        self.sections.get(y_index)
    }

    /// Set a block at world coordinates within this chunk.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block_id: u16) {
        if x < 0 || x >= 16 || z < 0 || z >= 16 || y < WORLD_BOTTOM_Y || y >= WORLD_HEIGHT {
            return;
        }
        let section_index = ((y - WORLD_BOTTOM_Y) / 16) as usize;
        let local_y = ((y - WORLD_BOTTOM_Y) % 16) as usize;
        if let Some(section) = self.sections.get_mut(section_index) {
            section.blocks[local_y][z as usize][x as usize] = block_id;
            section.is_empty = false;
        }
    }

    /// Get a block at world coordinates within this chunk.
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> u16 {
        if x < 0 || x >= 16 || z < 0 || z >= 16 || y < WORLD_BOTTOM_Y || y >= WORLD_HEIGHT {
            return AIR;
        }
        let section_index = ((y - WORLD_BOTTOM_Y) / 16) as usize;
        let local_y = ((y - WORLD_BOTTOM_Y) % 16) as usize;
        self.sections
            .get(section_index)
            .and_then(|s| Some(s.blocks[local_y][z as usize][x as usize]))
            .unwrap_or(AIR)
    }

    /// Flatten all blocks into a contiguous Vec<u16> for network serialization.
    /// Order: Y-major (bottom to top), then Z, then X.
    pub fn flatten(&self) -> Vec<u16> {
        let total = 16 * 16 * (WORLD_HEIGHT - WORLD_BOTTOM_Y) as usize;
        let mut out = Vec::with_capacity(total);
        for y in WORLD_BOTTOM_Y..WORLD_HEIGHT {
            for z in 0..16 {
                for x in 0..16 {
                    out.push(self.get_block(x, y, z));
                }
            }
        }
        out
    }
}

/// Generate a full chunk from WorldGen.
pub fn generate_chunk(gen: &WorldGen, chunk_x: i32, chunk_z: i32) -> Chunk {
    let mut chunk = Chunk::new(chunk_x, chunk_z);

    for x in 0..16 {
        for z in 0..16 {
            let wx = chunk_x * 16 + x;
            let wz = chunk_z * 16 + z;

            let biome = gen.noise.biome_at(wx, wz);
            let surface = gen.surface_height(wx, wz, &biome);

            for y in WORLD_BOTTOM_Y..WORLD_HEIGHT {
                let block_id = if y < surface - 3 {
                    STONE
                } else if y < surface {
                    DIRT
                } else if y == surface {
                    if biome.surface_block == GRASS_BLOCK && y <= SEA_LEVEL + 2 {
                        GRASS_BLOCK
                    } else {
                        biome.surface_block
                    }
                } else if y <= SEA_LEVEL && y > surface {
                    WATER
                } else {
                    AIR
                };

                chunk.set_block(x, y, z, block_id);
            }

            // Bedrock at bottom
            for y in WORLD_BOTTOM_Y..WORLD_BOTTOM_Y + 5 {
                chunk.set_block(x, y, z, BEDROCK);
            }

            // Cave carving
            for y in (WORLD_BOTTOM_Y + 5)..(WORLD_HEIGHT - 10) {
                let cave = gen.noise.cave_noise(wx, y, wz);
                if cave > 0.65 {
                    chunk.set_block(x, y, z, AIR);
                } else if cave > 0.55 && y < SEA_LEVEL {
                    chunk.set_block(x, y, z, WATER);
                }
            }
        }
    }

    chunk
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_flatten_size() {
        let chunk = Chunk::new(0, 0);
        let flat = chunk.flatten();
        assert_eq!(flat.len(), 16 * 16 * (WORLD_HEIGHT - WORLD_BOTTOM_Y) as usize);
    }

    #[test]
    fn chunk_set_get_block() {
        let mut chunk = Chunk::new(0, 0);
        chunk.set_block(5, 64, 5, STONE);
        assert_eq!(chunk.get_block(5, 64, 5), STONE);
        assert_eq!(chunk.get_block(0, 0, 0), AIR);
    }

    #[test]
    fn chunk_out_of_bounds_returns_air() {
        let chunk = Chunk::new(0, 0);
        assert_eq!(chunk.get_block(-1, 64, 0), AIR);
        assert_eq!(chunk.get_block(16, 64, 0), AIR);
        assert_eq!(chunk.get_block(0, -1, 0), AIR);
        assert_eq!(chunk.get_block(0, WORLD_HEIGHT, 0), AIR);
    }

    #[test]
    fn generate_chunk_is_deterministic() {
        let gen1 = WorldGen::new(42);
        let gen2 = WorldGen::new(42);
        let c1 = generate_chunk(&gen1, 0, 0);
        let c2 = generate_chunk(&gen2, 0, 0);
        assert_eq!(c1.flatten(), c2.flatten());
    }
}
