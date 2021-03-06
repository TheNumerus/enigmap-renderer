use rand::prelude::*;
use rand::rngs::StdRng;
use noise::{Fbm, NoiseFn, Seedable, Worley, Perlin};
use std::f32;

use crate::hexmap::HexMap;
use crate::hex::HexType;
use crate::generators::MapGen;

/// Generator that generates multiple islands
#[derive(Debug, Clone, Copy)]
pub struct Islands {
    seed: u32,
    using_seed: bool,
    pub ocean_distance: u32
}

impl Islands {
    /// Generates ice on top and bootom
    fn ice_pass<T>(&self, hex_map: &mut HexMap, gen: &T, noise_scale: f64, seed: u32)
        where T: NoiseFn<[f64; 2]>
    {
        // generate ice
        for hex in &mut hex_map.field {
            // hex specific fields
            let center = hex.center();
            let worley_val = gen.get([center.0 as f64 * noise_scale + seed as f64, center.1 as f64 * noise_scale]);
            let dst_to_edge = 1.0 - ((center.1 / hex_map.absolute_size_y - 0.5).abs() * 2.0);
            
            // make sure ice is certain to appear
            if hex.y == 0 || hex.y == (hex_map.size_y as i32 - 1) {
                hex.terrain_type = HexType::Ice;
            }
            // ice noise on top and bottom
            let noisy_dst_to_edge = dst_to_edge + (worley_val * 0.03) as f32;
            if noisy_dst_to_edge < 0.12 {
                hex.terrain_type = HexType::Ice;
            }
        }
        // clear up ice by removing some isalnds of ice and water
        for _ in 0..2 {
            self.clear_pass(hex_map, HexType::Ocean, HexType::Ice, 3);
            self.clear_pass(hex_map, HexType::Ice, HexType::Ocean, 3);
        }
    }


    /// Generates land
    fn land_pass<T>(&self, hex_map: &mut HexMap, gen: &T, noise_scale: f64, seed: u32)
        where T:NoiseFn<[f64; 2]>
    {
        // generate and clear up small islands
        for hex in &mut hex_map.field {
            if let HexType::Ocean = hex.terrain_type {
                let center = hex.center();
                let noise_val = gen.get([center.0 as f64 * noise_scale + seed as f64, center.1 as f64 * noise_scale]);
                if noise_val > 0.36 {
                    hex.terrain_type = HexType::Field;
                }
            }
        }
        for _ in 0..3 {
            self.clear_pass(hex_map, HexType::Field, HexType::Ocean, 3);
            self.clear_pass(hex_map, HexType::Ocean, HexType::Field, 3);
        }

        // create bigger landmasses
        // choose random points at centers of those landmasses
        let mut rng = StdRng::from_seed(self.seed_to_rng_seed(seed));
        for _ in 0..3 {
            // get first focus
            let x: f32 = rng.gen_range(0.0, hex_map.absolute_size_x);
            let y: f32 = rng.gen_range(0.1, 0.9) * hex_map.absolute_size_y;
            let first_focus = (x,y);

            // get aproximate center of the map
            let center = (hex_map.absolute_size_x / 2.0 + rng.gen_range(-10.0, 10.0), hex_map.absolute_size_y / 2.0 + rng.gen_range(-10.0, 10.0));

            // get unit vector with direction from first focus to center
            let mut vector = (center.0 - first_focus.0, center.1 - first_focus.1);
            let len = (vector.0.powi(2) + vector.1.powi(2)).sqrt();
            vector.0 /= len;
            vector.1 /= len;

            // multiply it by random value and get second focus
            let island_len: f32 = rng.gen_range(hex_map.absolute_size_y / 4.0, hex_map.absolute_size_y / 2.5);
            let second_focus = (first_focus.0 + vector.0 * island_len, first_focus.1 + vector.1 * island_len);

            // between them is center of the big island
            let center_focus = ((first_focus.0 + second_focus.0) / 2.0, (first_focus.1 + second_focus.1) / 2.0);

            // now generate landmasses
            for hex in &mut hex_map.field {
                // skip tiles that aren't water
                match hex.terrain_type {
                    HexType::Ocean => {},
                    _ => continue
                };
                let center = hex.center();
                let noise_val = gen.get([center.0 as f64 * noise_scale + seed as f64, center.1 as f64 * noise_scale]);
                // get distances to selecte points and generate islands from those
                let first_dst = ((center.0 - first_focus.0).powi(2) + (center.1 - first_focus.1).powi(2)).sqrt();
                let second_dst = ((center.0 - second_focus.0).powi(2) + (center.1 - second_focus.1).powi(2)).sqrt();
                let center_dst = ((center.0 - center_focus.0).powi(2) + (center.1 - center_focus.1).powi(2)).sqrt() * 0.6;
                let elipse_dst = f32::min(center_dst, f32::min(first_dst, second_dst)) / hex_map.absolute_size_x * 100.0;
                if (noise_val as f32 * 3.0 + elipse_dst) < 4.0 {
                    hex.terrain_type = HexType::Field;
                }
            }
        }
    }

    /// Changes tiles with `HexType::FIELD` to something different based on position
    fn decorator_pass<T>(&self, hex_map: &mut HexMap, gen: &T, noise_scale: f64, seed: u32)
        where T:NoiseFn<[f64; 2]>
    {

        let mut rng = StdRng::from_seed(self.seed_to_rng_seed(seed));
        for hex in &mut hex_map.field {
            // skip everything thats not land and generate mountains
            match hex.terrain_type {
                HexType::Field => {
                    if rng.gen::<f32>() < 0.04 {
                        hex.terrain_type = HexType::Mountain;
                        continue;
                    }
                }, 
                _ => continue
            };

            let center = hex.center();
            let dst_to_edge = 1.0 - ((center.1 / hex_map.absolute_size_y - 0.5).abs() * 2.0);
            let noise_val = gen.get([center.0 as f64 * noise_scale + seed as f64, center.1 as f64 * noise_scale]);
            let temperature = 70.0 * dst_to_edge - 20.0 + noise_val as f32 * 5.0;
            hex.terrain_type = if temperature < -5.0 {
                HexType::Tundra
            } else if temperature > -5.0 && temperature < 25.0 && noise_val > -0.6 {
                HexType::Forest
            } else if temperature > 35.0 && noise_val > -0.6 {
                HexType::Desert
            } else {
                HexType::Field
            };
        }

        // generate jungles by computing vector field for wind
        // areas which have wind pointed to ocean will be deserts
        // and areas with wind blowing to them will be jungles
        let mut wind_field: Vec<(f32, f32)> = Vec::with_capacity(hex_map.field.len());
        for hex in &hex_map.field {
            let center = hex.center();
            let noise_val_x = gen.get([center.0 as f64 * 0.15 * noise_scale + seed as f64, center.1 as f64 * 0.15 * noise_scale]) as f32;
            let noise_val_y = gen.get([center.0 as f64 * 0.15 * noise_scale - seed as f64, center.1 as f64 * 0.15 * noise_scale]) as f32;
            let len = (noise_val_x.powi(2) + noise_val_y.powi(2)).sqrt() as f32;
            wind_field.push((noise_val_x / len, noise_val_y / len));
        }

        let old_map = hex_map.clone();
        for (index, hex) in hex_map.field.iter_mut().enumerate() {
            // skip not deserts
            match hex.terrain_type {
                HexType::Desert => {},
                _ => continue
            }
            let (x_wind, y_wind) = wind_field[index];

            let center = hex.center();

            let target_x = center.0 + x_wind * old_map.get_avg_size() as f32 * 0.2;
            let target_y = center.1 + y_wind * old_map.get_avg_size() as f32 * 0.2;

            let target_hex_index = old_map.get_closest_hex_index(target_x, target_y);

            match old_map.field[target_hex_index].terrain_type {
                HexType::Water | HexType::Ocean => {
                    hex.terrain_type = HexType::Jungle;
                },
                _ => {}
            } 
            /* debug wind direction
            if x_wind > 0.0 && y_wind > 0.0 {
                hex.terrain_type = HexType::Water;
            } else if x_wind < 0.0 && y_wind > 0.0 {
                hex.terrain_type = HexType::Field;
            } else if x_wind > 0.0 && y_wind < 0.0 {
                hex.terrain_type = HexType::Ice;
            } else {
                hex.terrain_type = HexType::Desert;
            }*/
        }
    }

    /// Generates oceans by changing `HexType::Ocean` tiles into `HexType::Water`
    /// 
    /// Uses same generator as land pass for better ocean generation
    fn ocean_pass<T>(&self, hex_map: &mut HexMap, gen: &T, noise_scale: f64, seed: u32)
        where T:NoiseFn<[f64; 2]>
    {
        let mut land_tiles = 0;
        // copy only land tiles into 2d array
        let mut old_field: Vec<Vec<(i32, i32)>> = vec![Vec::new(); hex_map.size_y as usize];
        for (line_num, line) in &mut hex_map.field.chunks_exact(hex_map.size_x as usize).enumerate() {
            for hex in line {
                match hex.terrain_type {
                    HexType::Water | HexType::Ice | HexType::Ocean => continue,
                    _ => {
                        // copy only coordinates
                        old_field[line_num].push((hex.x, hex.y));
                        land_tiles+=1;
                    }
                };
            }
        }

        // don't even do ocean pass if there isn't land
        if land_tiles == 0 {
            return;
        }

        'hex: for hex in &mut hex_map.field {
            // skip everything thats not ocean
            match hex.terrain_type {
                HexType::Ocean => {}, 
                _ => continue
            };
            let mut dst_to_land = u32::max_value();
            let hex_center = hex.center();
            let noise_val = gen.get([hex_center.0 as f64 * noise_scale + seed as f64, hex_center.1 as f64 * noise_scale]);

            // get upper and lower boundary on lines in which can land be found
            let min_y = (hex.y - self.ocean_distance as i32).max(0) as usize;
            let max_y = (hex.y + self.ocean_distance as i32).min(hex_map.size_y as i32 - 1) as usize;

            // get distance to land
            for line in &old_field[min_y..=max_y] {
                let mut distance_in_line = u32::max_value();
                for other in line {
                    let dst = hex.distance_to( other.0, other.1);
                    // if the second hex on line is further away, don't even compute the whole line
                    if dst > distance_in_line {
                        break;
                    }
                    distance_in_line = dst;
                    if dst < dst_to_land {
                        dst_to_land = dst;
                        // spawn water and make sure we have at least one tile
                        if dst_to_land <= self.ocean_distance && noise_val >= 0.14 || dst_to_land == 1 {
                            hex.terrain_type = HexType::Water;
                            continue 'hex;
                        }
                    }
                }
            }
        }
        //clear that up a little bit
        self.clear_pass(hex_map, HexType::Ocean, HexType::Water, 3);
    }
}

impl Default for Islands {
    fn default() -> Islands {
        Islands{seed: 0, using_seed: false, ocean_distance: 5}
    }
}

impl MapGen for Islands {
    fn generate(&self, hex_map: &mut HexMap) {
        hex_map.fill(HexType::Ocean);

        // init generators
        let w = Worley::new();
        let f = Fbm::new();
        let p = Perlin::new();
        let seed = if self.using_seed {
            self.seed
        } else {
            random::<u32>()
        };

        debug_println!("seed: {:?}", seed);
        w.set_seed(seed);
        p.set_seed(seed);
        w.enable_range(true);

        // noise scale
        let noise_scale = 60.0 / hex_map.absolute_size_x as f64;
        let land_noise_scale = 8.0 / hex_map.absolute_size_x as f64;
        
        self.ice_pass(hex_map, &w, noise_scale, seed);
        debug_println!("Ice generated");
        self.land_pass(hex_map, &f, land_noise_scale, seed);
        debug_println!("Land generated");
        self.decorator_pass(hex_map, &p, noise_scale, seed);
        debug_println!("Land features generated");
        self.ocean_pass(hex_map, &f, land_noise_scale, seed);
        debug_println!("Oceans generated");
    }

    fn set_seed(&mut self, seed: u32) {
        self.using_seed = true;
        self.seed = seed;
    }

    fn reset_seed(&mut self) {
        self.using_seed = false;
    }
}