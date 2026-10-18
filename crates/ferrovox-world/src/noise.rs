//! # Noise generation
//!
//! Gradient noise (Simplex-style) used for terrain height, biomes, and cave
//! carving. All noise is derived from a single 64-bit seed so the same seed
//! produces identical terrain on both Java and Bedrock clients.



/// A complete set of noise generators for world generation.
#[derive(Debug, Clone)]
pub struct NoiseSet {
    seed: u64,
    // Permutation tables for gradient noise (stored as u32)
    perm_x: [u32; 512],
    perm_z: [u32; 512],
    perm_y: [u32; 512],
}

impl NoiseSet {
    /// Create a new noise set from a seed.
    pub fn new(seed: u64) -> Self {
        let mut perm_x = [0u32; 512];
        let mut perm_z = [0u32; 512];
        let mut perm_y = [0u32; 512];

        for i in 0..256 {
            perm_x[i] = hash_u32(seed.wrapping_add(i as u64).wrapping_mul(0x9E3779B97F4A7C15));
            perm_z[i] = hash_u32(seed.wrapping_add(i as u64).wrapping_mul(0xBF58476D1CE4E5B9));
            perm_y[i] = hash_u32(seed.wrapping_add(i as u64).wrapping_mul(0x94D049BB133111EB));
        }
        // Duplicate for easy wrapping
        for i in 256..512 {
            perm_x[i] = perm_x[i - 256];
            perm_z[i] = perm_z[i - 256];
            perm_y[i] = perm_y[i - 256];
        }

        Self { seed, perm_x, perm_z, perm_y }
    }

    /// Continentalness noise — drives large-scale terrain shape.
    pub fn continentalness(&self, x: i32, z: i32) -> f32 {
        let fx = x as f32 / 400.0;
        let fz = z as f32 / 400.0;
        simplex2(&self.perm_x, fx, fz) * 0.5
            + simplex2(&self.perm_z, fx * 2.0, fz * 2.0) * 0.25
            + simplex2(&self.perm_x, fx * 4.0, fz * 4.0) * 0.125
    }

    /// Erosion noise — drives terrain roughness.
    pub fn erosion(&self, x: i32, z: i32) -> f32 {
        let fx = x as f32 / 300.0;
        let fz = z as f32 / 300.0;
        simplex2(&self.perm_z, fx, fz) * 0.6
            + simplex2(&self.perm_y, fx * 2.0, fz * 2.0) * 0.3
    }

    /// Weirdness noise — drives unusual terrain features.
    pub fn weirdness(&self, x: i32, z: i32) -> f32 {
        let fx = x as f32 / 500.0;
        let fz = z as f32 / 500.0;
        simplex2(&self.perm_x, fx, fz) * 0.4
    }

    /// Temperature noise — drives biome temperature axis.
    pub fn temperature(&self, x: i32, z: i32) -> f32 {
        let fx = x as f32 / 800.0;
        let fz = z as f32 / 800.0;
        simplex2(&self.perm_y, fx, fz) * 0.5
            + simplex2(&self.perm_x, fx * 4.0, fz * 4.0) * 0.25
    }

    /// Humidity noise — drives biome humidity axis.
    pub fn humidity(&self, x: i32, z: i32) -> f32 {
        let fx = x as f32 / 700.0;
        let fz = z as f32 / 700.0;
        simplex2(&self.perm_z, fx, fz) * 0.5
            + simplex2(&self.perm_y, fx * 3.0, fz * 3.0) * 0.25
    }

    /// Cave noise — 3D noise for cave carving.
    pub fn cave_noise(&self, x: i32, y: i32, z: i32) -> f32 {
        let fx = x as f32 / 80.0;
        let fy = y as f32 / 80.0;
        let fz = z as f32 / 80.0;
        simplex3(&self.perm_x, fx, fy, fz) * 0.5
            + simplex3(&self.perm_z, fx * 2.0, fy * 2.0, fz * 2.0) * 0.25
            + simplex3(&self.perm_y, fx * 4.0, fy * 4.0, fz * 4.0) * 0.125
    }

    /// Biome selection from climate axes.
    pub fn biome_at(&self, x: i32, z: i32) -> crate::Biome {
        let temp = self.temperature(x, z);
        let hum = self.humidity(x, z);
        let cont = self.continentalness(x, z);

        crate::biome_at(temp, hum, cont)
    }
}

/// Hash a u64 to a u32.
fn hash_u32(mut v: u64) -> u32 {
    v = v.wrapping_mul(0xFF51AFD7ED558CCD);
    v ^= v >> 32;
    v = v.wrapping_mul(0xC4CEB9FE1A85EC53);
    v ^= v >> 32;
    v as u32
}

/// Wrap any usize into [0, 511] for safe array indexing.
#[inline(always)]
fn wrap512(v: usize) -> usize {
    v % 512
}

/// 2D Simplex-like gradient noise.
fn simplex2(perm: &[u32; 512], x: f32, y: f32) -> f32 {
    let x = x;
    let y = y;
    let s = (x + y) * 0.366025404; // (sqrt(3)-1)/2
    let i = (x + s).floor() as i32;
    let j = (y + s).floor() as i32;
    let t = (i + j) as f32 * 0.211324865; // (3-sqrt(3))/6
    let x0 = x - (i as f32 - t);
    let y0 = y - (j as f32 - t);

    let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };

    let x1 = x0 - i1 as f32 + 0.211324865;
    let y1 = y0 - j1 as f32 + 0.211324865;
    let x2 = x0 - 1.0 + 2.0 * 0.211324865;
    let y2 = y0 - 1.0 + 2.0 * 0.211324865;

    let ii = i.rem_euclid(256) as usize;
    let jj = j.rem_euclid(256) as usize;

    // All indices wrapped to [0, 511] to prevent out-of-bounds access
    let p0 = wrap512(perm[jj] as usize);
    let idx0 = wrap512(ii + p0);
    let g0 = grad2(perm[idx0] as i32, x0, y0);

    let p1 = wrap512(perm[wrap512(jj + j1 as usize)] as usize);
    let idx1 = wrap512(ii + i1 as usize + p1);
    let g1 = grad2(perm[idx1] as i32, x1, y1);

    let p2 = wrap512(perm[wrap512(jj + 1)] as usize);
    let idx2 = wrap512(ii + 1usize + p2);
    let g2 = grad2(perm[idx2] as i32, x2, y2);

    let t0 = 0.5 - x0 * x0 - y0 * y0;
    let t1 = 0.5 - x1 * x1 - y1 * y1;
    let t2 = 0.5 - x2 * x2 - y2 * y2;

    let mut n = 0.0;
    if t0 > 0.0 { n += t0 * t0 * t0 * t0 * g0; }
    if t1 > 0.0 { n += t1 * t1 * t1 * t1 * g1; }
    if t2 > 0.0 { n += t2 * t2 * t2 * t2 * g2; }

    n * 40.0
}

/// 3D Simplex-like gradient noise.
fn simplex3(perm: &[u32; 512], x: f32, y: f32, z: f32) -> f32 {
    let x = x;
    let y = y;
    let z = z;
    let s = (x + y + z) * 0.333333333;
    let i = (x + s).floor() as i32;
    let j = (y + s).floor() as i32;
    let k = (z + s).floor() as i32;
    let t = (i + j + k) as f32 * 0.166666667;
    let x0 = x - (i as f32 - t);
    let y0 = y - (j as f32 - t);
    let z0 = z - (k as f32 - t);

    // Determine which simplex we're in
    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0 { (1, 0, 0, 1, 1, 0) }
        else if x0 >= z0 { (1, 0, 0, 1, 0, 1) }
        else { (0, 0, 1, 1, 0, 1) }
    } else {
        if y0 < z0 { (0, 0, 1, 0, 1, 1) }
        else if x0 < z0 { (0, 1, 0, 0, 1, 1) }
        else { (0, 1, 0, 1, 1, 0) }
    };

    let x1 = x0 - i1 as f32 + 0.166666667;
    let y1 = y0 - j1 as f32 + 0.166666667;
    let z1 = z0 - k1 as f32 + 0.166666667;
    let x2 = x0 - i2 as f32 + 0.333333333;
    let y2 = y0 - j2 as f32 + 0.333333333;
    let z2 = z0 - k2 as f32 + 0.333333333;
    let x3 = x0 - 1.0 + 0.5;
    let y3 = y0 - 1.0 + 0.5;
    let z3 = z0 - 1.0 + 0.5;

    let ii = i.rem_euclid(256) as usize;
    let jj = j.rem_euclid(256) as usize;
    let kk = k.rem_euclid(256) as usize;

    // All indices wrapped to [0, 511] to prevent out-of-bounds access
    let p0 = wrap512(perm[kk] as usize);
    let idx0 = wrap512(ii + p0);
    let g0 = grad3(perm[idx0] as i32, x0, y0, z0);

    let p1b = wrap512(perm[wrap512(kk + k1 as usize)] as usize);
    let p2 = wrap512(perm[wrap512(jj + j1 as usize + p1b)] as usize);
    let idx1 = wrap512(ii + i1 as usize + p2);
    let g1 = grad3(perm[idx1] as i32, x1, y1, z1);

    let p2b = wrap512(perm[wrap512(kk + k2 as usize)] as usize);
    let p3 = wrap512(perm[wrap512(jj + j2 as usize + p2b)] as usize);
    let idx2 = wrap512(ii + i2 as usize + p3);
    let g2 = grad3(perm[idx2] as i32, x2, y2, z2);

    let p3b = wrap512(perm[wrap512(kk + 1)] as usize);
    let p4 = wrap512(perm[wrap512(jj + 1 + p3b)] as usize);
    let idx3 = wrap512(ii + 1usize + p4);
    let g3 = grad3(perm[idx3] as i32, x3, y3, z3);

    let t0 = 0.6 - x0 * x0 - y0 * y0 - z0 * z0;
    let t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
    let t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
    let t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;

    let mut n = 0.0;
    if t0 > 0.0 { n += t0 * t0 * t0 * t0 * g0; }
    if t1 > 0.0 { n += t1 * t1 * t1 * t1 * g1; }
    if t2 > 0.0 { n += t2 * t2 * t2 * t2 * g2; }
    if t3 > 0.0 { n += t3 * t3 * t3 * t3 * g3; }

    n * 32.0
}

/// Gradient dot product for 2D.
fn grad2(hash: i32, x: f32, y: f32) -> f32 {
    match hash & 0x3 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        _ => -x - y,
    }
}

/// Gradient dot product for 3D.
fn grad3(hash: i32, x: f32, y: f32, z: f32) -> f32 {
    match hash & 0xF {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        11 => -y - z,
        12 => y + x,
        13 => -y + x,
        14 => y - x,
        _ => -y - x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_deterministic() {
        let n1 = NoiseSet::new(42);
        let n2 = NoiseSet::new(42);
        assert_eq!(n1.continentalness(10, 20), n2.continentalness(10, 20));
        assert_eq!(n1.erosion(10, 20), n2.erosion(10, 20));
    }

    #[test]
    fn different_seeds_different_noise() {
        let n1 = NoiseSet::new(1);
        let n2 = NoiseSet::new(2);
        assert_ne!(n1.continentalness(10, 20), n2.continentalness(10, 20));
    }

    #[test]
    fn noise_in_range() {
        let n = NoiseSet::new(12345);
        for x in -100..100 {
            for z in -100..100 {
                let c = n.continentalness(x, z);
                assert!(c >= -1.0 && c <= 1.0, "continentalness {} out of range at {},{}", c, x, z);
                let e = n.erosion(x, z);
                assert!(e >= -1.0 && e <= 1.0, "erosion {} out of range at {},{}", e, x, z);
            }
        }
    }

    #[test]
    fn noise_handles_negative_coords() {
        let n = NoiseSet::new(42);
        // Should not panic on negative coordinates
        let _ = n.continentalness(-100, -200);
        let _ = n.erosion(-50, -50);
        let _ = n.cave_noise(-10, -20, -30);
    }
}
