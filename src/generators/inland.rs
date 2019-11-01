use crate::generators::MapGen;
use crate::hexmap::HexMap;
use crate::hex::HexType;

use rand::prelude::*;

use std::f32;

#[derive(Debug, Clone, Copy)]
pub struct Inland {
    seed: Option<u32>,
    wrap_map: bool,
    pub temperature: InlandParam,
    pub flatness: InlandParam,
    pub humidity: InlandParam,
    pub region_size: InlandParam
}

impl Inland {
    pub fn set_wrap_map(&mut self, value: bool) {
        self.wrap_map = value;
    }

    fn get_region_count(&self, hex_map: &HexMap) -> u32 {
        let area = hex_map.get_area();

        // range from <35; 95>
        let area_size = (f32::from(self.region_size) * 60.0 + 35.0) as u32;
        
        (area / area_size).max(1)
    }

    fn generate_centers(&self, hex_map: &HexMap, rng: &mut StdRng) -> Vec<usize> {
        let region_count = self.get_region_count(hex_map);

        let mut probabilities = vec![1.0; hex_map.get_area() as usize];
        let mut total_probability = hex_map.get_area() as f32;

        let distance = (hex_map.get_avg_size() as f32 * 0.2) as u32;
        let strength = 1.1;

        let get_mult = |dist: f32| {
            ((dist - 2.0).max(0.0) / strength).log10().min(1.0).max(0.0)
        };

        // make centers less probable on top and bottom
        let fadeout = (hex_map.size_y as f32 * 0.1) as u32;
        for i in 0..fadeout {
            let fade_strength = (i as f32 / fadeout as f32).sqrt();
            for x in 0..hex_map.size_x {
                // top
                let index = (x + i * hex_map.size_x) as usize;
                let temp = probabilities[index];
                probabilities[index] *= fade_strength;
                total_probability -= temp - probabilities[index];

                // bottom
                let index = (x + (hex_map.size_y - 1 - i) * hex_map.size_x) as usize;
                let temp = probabilities[index];
                probabilities[index] *= fade_strength;
                total_probability -= temp - probabilities[index];
            }
        }

        let mut centers = vec![];

        for _i in 0..region_count {
            let random_number = rng.gen::<f32>() * total_probability;
            let mut total = 0.0;
            let mut hex = 0;
            for k in 0..probabilities.len() {
                if total < random_number {
                    total += probabilities[k];
                } else {
                    hex = k;
                    break;
                }
            };
            centers.push(hex);
            total_probability -= probabilities[hex];
            probabilities[hex] = 0.0;
            // now update probabilities
            for r in 1..distance {
                let ring = hex_map.field[hex].get_ring(hex_map, r);
                let mult = get_mult(r as f32);
                for (hex_x, hex_y) in ring {
                    let index = hex_map.coords_to_index(hex_x, hex_y);
                    let index = match index {
                        Some(val) => val,
                        None => continue
                    };
                    let old_prob = probabilities[index];
                    probabilities[index] *= mult;
                    total_probability -= old_prob - probabilities[index];
                }
            }
        }

        centers
    }

    fn generate_regions(&self, rng: &mut StdRng, hex_map: &mut HexMap, centers: &Vec<usize>) -> Regions {
        let mut regions = Regions::new(centers.len());
        let mut frontiers: Vec<Vec<usize>> = Vec::new();

        for (i, &reg) in centers.iter().enumerate() {
            regions.regions[i].center = reg;
            let neighbours = hex_map.field[reg].get_neighbours(&hex_map);

            let mut frontier = Vec::new();

            for (x,y) in neighbours {
                let index = hex_map.coords_to_index(x,y).unwrap();
                frontier.push(index);
            }
            frontiers.push(frontier);
        }

        let mut hexes_to_set = hex_map.get_area() - centers.len() as u32;
        let mut hexes_set = vec![None; hex_map.get_area() as usize];
        for (index, center) in centers.iter().enumerate() {
            hexes_set[*center] = Some(index);
        }

        'filler: loop {
            for i in 0..regions.regions.len() {
                if hexes_to_set == 0 {
                    break 'filler;
                }

                if frontiers[i].is_empty() {
                    continue;
                }
                let hex_index = rng.gen_range(0, frontiers[i].len());
                let hex = frontiers[i][hex_index];

                if let Some(val) = hexes_set[hex] {
                    if val != i {
                        frontiers[i].remove(hex_index);
                        continue;
                    }
                }

                frontiers[i].remove(hex_index);

                regions.regions[i].hexes.push(hex);
                hexes_set[hex] = Some(i);

                let neighbours = hex_map.field[hex].get_neighbours(&hex_map);

                for (x,y) in neighbours {
                    let index = hex_map.coords_to_index(x,y).unwrap();
                    if let None = hexes_set[index] {
                        if !frontiers[i].contains(&index) {
                            frontiers[i].push(index);
                            hexes_set[index] = Some(i);
                        }
                    }
                }

                hexes_to_set -= 1;
            }
        }

        regions
    }

    fn decorate_reg(&self, hex_map: &mut HexMap, reg: &Region) {
        hex_map.field[reg.center].terrain_type = HexType::Debug(0.1);

        let base = Inland::search_type(reg.temperature, reg.flatness, reg.humidity);
        //dbg!(&base);
        // create base
        for hex in &reg.hexes {
            if reg.water_region {
                hex_map.field[*hex].terrain_type = HexType::Water;
            } else {
                hex_map.field[*hex].terrain_type = base;
            }
        }
    }

    fn search_type(temp: f32, flat: f32, hum: f32) -> HexType {
        let mut smallest_dist = f32::MAX;
        let mut best_match = HexType::Debug2d(1.0 , 0.0);
        for (hex_type, x, y, z) in &TYPE_COORDS {
            let dist = (((x - temp).powi(2) + (y - flat).powi(2)).sqrt() + (z - hum).powi(2)).sqrt();
            if dist < smallest_dist {
                smallest_dist = dist;
                best_match = *hex_type;
            }
        }

        best_match
    }
}

impl MapGen for Inland {
    fn generate(&self, hex_map: &mut HexMap) {
        let seed = match self.seed {
            Some(val) => val,
            None => random::<u32>()
        };
        
        let mut rng = StdRng::from_seed(self.seed_to_rng_seed(seed));

        let centers = self.generate_centers(hex_map, &mut rng);

        let mut regions = self.generate_regions(&mut rng, hex_map, &centers);

        // create region parameters
        for region in &mut regions.regions {
            let center = hex_map.field[region.center];
            let coords = (center.center_x, center.center_y);
            let norm_coords = (coords.0 / hex_map.absolute_size_x, coords.1 / hex_map.absolute_size_y);

            let rand: f32 = rng.gen_range(-1.0, 1.0);
            let temp = 0.10 * rand + f32::from(self.temperature) + 0.2 * -(((norm_coords.1 - 0.5) * 2.0).abs() + 0.5);
            region.temperature = temp;
            region.humidity = f32::from(self.humidity) + rng.gen_range(-1.0, 1.0) * 0.15;
            region.flatness = f32::from(self.flatness) + rng.gen_range(-1.0, 1.0) * 0.15;
        }

        for reg in &regions.regions {
            self.decorate_reg(hex_map, reg);
        }
    }

    fn set_seed(&mut self, seed: u32) {
        self.seed = Some(seed);
    }

    fn reset_seed(&mut self) {
        self.seed = None;
    }
}

impl Default for Inland {
    fn default() -> Inland {
        Inland{
            seed: None,
            wrap_map: true,
            temperature: InlandParam::Medium,
            flatness: InlandParam::Medium,
            humidity: InlandParam::Medium,
            region_size: InlandParam::Medium
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum InlandParam {
    Low,
    Medium,
    High,
    Custom(f32)
}

impl From<InlandParam> for f32 {
    fn from(t: InlandParam) -> f32 {
        match t {
            InlandParam::Low => 0.15,
            InlandParam::Medium => 0.5,
            InlandParam::High => 0.85,
            InlandParam::Custom(val) => val.min(1.0).max(0.0)
        }
    }
}

impl Default for InlandParam {
    fn default() -> InlandParam {
        InlandParam::Medium
    }
}

#[derive(Debug, Clone)]
struct Region {
    center: usize,
    temperature: f32,
    humidity: f32,
    flatness: f32,
    water_region: bool,
    hexes: Vec<usize>
}

impl Default for Region {
    fn default() -> Self {
        Region{center: 0, temperature: 0.5, humidity: 0.5, flatness: 0.5, water_region: false, hexes: Vec::new()}
    }
}

#[derive(Debug, Clone)]
struct Regions {
    regions: Vec<Region>
}

impl Regions {
    pub fn new(len: usize) -> Regions {
        let mut regions = Vec::with_capacity(len);

        for _reg in 0..len {
            regions.push(Region::default());
        }

        Regions{regions}
    }
}

// (type, temp, flatness, humidity)
const TYPE_COORDS: [(HexType, f32, f32, f32); 10] = [
    (HexType::Field, 0.6, 0.4, 0.5),
    (HexType::Forest, 0.4, 0.6, 0.5),
    (HexType::Desert, 0.8, 0.4, 0.1),
    (HexType::Tundra, 0.2, 0.5, 0.5),
    (HexType::Water, 0.5, 0.5, 1.0),
    (HexType::Mountain, 0.5, 1.0, 0.5),
    (HexType::Ice, 0.0, 0.5, 0.5),
    (HexType::Jungle, 0.8, 0.5, 0.8),
    (HexType::Swamp, 0.5, 0.5, 0.95),
    (HexType::Grassland, 0.45, 0.6, 0.5),
];
