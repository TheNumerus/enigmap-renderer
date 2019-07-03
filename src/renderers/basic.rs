use std::time::Instant;

use rand::prelude::*;

use crate::hexmap::HexMap;
use crate::hex::{Hex, HexType};
use crate::renderers::{Image, Renderer, ColorMode};
use crate::renderers::colors::ColorMap;

// 4x4 samples is overkill
const SUPERSAMPLE_FACTOR: u32 = 2;

/// Software renderer
/// 
pub struct Basic {
    /// Size of `Hex` on X axis in pixels
    multiplier: f32,
    /// Should the map repeat on the X axis
    wrap_map: bool,
    /// Randomize colors slightly
    randomize_colors: bool,
    /// Use anti-aliasing when rendering
    antialiasing: bool,
    /// Colormap used when rendering
    pub colors: ColorMap
}

impl Basic {
    pub fn render_polygon (&self, points: &[(f32, f32)], img: &mut Image, color: [u8;3]) {
        if points.len() < 3 {
            return;
        }

        let mut min_x = points[0].0;
        let mut min_y = points[0].1;

        let mut max_x = points[0].0;
        let mut max_y = points[0].1;

        for point in &points[1..] {
            min_x = min_x.min(point.0);
            min_y = min_y.min(point.1);
            max_x = max_x.max(point.0);
            max_y = max_y.max(point.1);
        };

        max_x = max_x.min(img.width as f32);
        max_y = max_y.min(img.height as f32);

        min_x = min_x.max(0.0);
        min_y = min_y.max(0.0);


        // properly round float coordinates 
        let min_x = min_x.max(0.0).min(img.width as f32 - 1.0).round() as i32;
        let min_y = min_y.max(0.0).min(img.height as f32 - 1.0).round() as i32;
        let max_x = max_x.max(0.0).min(img.width as f32 - 1.0).round() as i32;
        let max_y = max_y.max(0.0).min(img.height as f32 - 1.0).round() as i32;

        let mut deltas: Vec<(f32, f32)> = Vec::with_capacity(points.len());
        let mut edges: Vec<f32> = Vec::with_capacity(points.len());

        for i in 0..points.len() {
            deltas.push((points[(i + 1) % points.len()].0 - points[i].0, points[(i + 1) % points.len()].1 - points[i].1));
            edges.push(((min_x as f32 + 0.5 - points[i].0) * deltas[i].1) - ((min_y as f32 + 0.5 - points[i].1) * deltas[i].0));
        }

        for y in (min_y)..=(max_y) {
            let is_reversed = ((y - min_y) % 2) != 0;
            let x_range: Box<dyn Iterator<Item = i32>> = if is_reversed {
                Box::new((min_x..=max_x).rev())
            } else {
                Box::new(min_x..=max_x)
            };

            for (x_index, x) in x_range.enumerate() {
                let mut in_triangle = true;
                for edge in &edges {
                    if *edge < 0.0 {
                        in_triangle = false;
                        break;
                    }
                }
                if in_triangle {
                    img.put_pixel(x as u32, y as u32, color);
                }

                // dont add offset if the tested pixel is last on line
                if x_index as i32 != (min_x - max_x).abs() {
                    for (index, edge) in edges.iter_mut().enumerate() {
                        if is_reversed {
                            *edge -= deltas[index].1;
                        } else {
                            *edge += deltas[index].1;
                        }
                    }
                }
            }

            for (index, edge) in edges.iter_mut().enumerate() {
                *edge -= deltas[index].0;
            }
        }
    }

    fn render_hex_to_image (&self, points: &[(f32, f32);6], img: &mut Image, color: [u8;3], is_bottom_row: bool) {
        // points are in this order
        //     0
        //  1     5
        //  2     4
        //     3

        // clip with image edges
        // properly round float coordinates 
        let min_x = points[1].0.max(0.0).min(img.width as f32 - 1.0).round() as i32;
        let min_y = points[0].1.max(0.0).min(img.height as f32 - 1.0).round() as i32;
        let max_x = points[5].0.max(0.0).min(img.width as f32 - 1.0).round() as i32;
        let max_y = points[3].1.max(0.0).min(img.height as f32 - 1.0).round() as i32;

        if min_x == max_x || min_y == max_y {
            return;
        }

        let is_cut = min_x == 0 || max_x == (img.width as i32 - 1);

        let mut deltas: [(f32, f32);4] = [(0.0, 0.0); 4];
        let mut edges: [f32;4] = [0.0; 4];
        let point_indices: [usize; 4] = [0,2,3,5];

        for i in 0..4 {
            deltas[i] = (points[(point_indices[i] + 1) % 6].0 - points[point_indices[i]].0, points[(point_indices[i] + 1) % 6].1 - points[point_indices[i]].1);
            edges[i] = ((min_x as f32 + 0.5 - points[point_indices[i]].0) * deltas[i].1) - ((min_y as f32 + 0.5 - points[point_indices[i]].1) * deltas[i].0);
        }

        let check_inside = |edges: &mut[f32]| {
            for edge in edges {
                if *edge < 0.0 {
                    return false;
                }
            }
            true
        };

        let mut line_state = LineState::BeforeHex;

        let mut middle_start = max_y;
        let mut middle_start_reversed = false;

        // render top
        'lines: for y in (min_y)..=(max_y) {
            let is_reversed = ((y - min_y) % 2) != 0;

            if is_reversed {
                for (x_index, x) in (min_x..=max_x).rev().enumerate() {
                    let in_hex = check_inside(&mut edges);
                    line_state.update(in_hex);
                    
                    if in_hex {
                        // if the first pixel on line is in hex, the whole line is
                        // don't use this rule on edges
                        if x_index == 0 && !is_cut {
                            middle_start = y;
                            middle_start_reversed = true;
                            line_state.reset();
                            break 'lines;
                        }
                        img.put_pixel(x as u32, y as u32, color);
                    } else {
                        // skip to the end of the line
                        match line_state {
                            LineState::AfterHex => {
                                // add all deltas at once
                                for (index, edge) in edges.iter_mut().enumerate() {
                                    *edge -= (max_x - min_x - x_index as i32) as f32 * deltas[index].1;
                                }
                                break
                            },
                            _ => {}
                        }
                    }

                    // dont add offset if the tested pixel is last on line
                    if x_index as i32 != (max_x - min_x) {
                        for (index, edge) in edges.iter_mut().enumerate() {
                            *edge -= deltas[index].1;
                        }
                    }
                }
            } else {
                for (x_index, x) in (min_x..=max_x).enumerate() {
                    let in_hex = check_inside(&mut edges);
                    line_state.update(in_hex);
                    
                    if in_hex {
                        // if the first pixel on line is in hex, the whole line is
                        // don't use this rule on edges
                        if x_index == 0 && !is_cut {
                            middle_start = y;
                            line_state.reset();
                            break 'lines;
                        }
                        img.put_pixel(x as u32, y as u32, color);
                    } else {
                        // skip to the end of the line
                        match line_state {
                            LineState::AfterHex => {
                                // add all deltas at once
                                for (index, edge) in edges.iter_mut().enumerate() {
                                    *edge += (max_x - min_x - x_index as i32) as f32 * deltas[index].1;
                                }
                                break
                            },
                            _ => {}
                        }
                    }

                    // dont add offset if the tested pixel is last on line
                    if x_index as i32 != (max_x - min_x) {
                        for (index, edge) in edges.iter_mut().enumerate() {
                            *edge += deltas[index].1;
                        }
                    }
                }
            }

            line_state.reset();

            for (index, edge) in edges.iter_mut().enumerate() {
                *edge -= deltas[index].0;
            }
        }

        // cut hexes are rendered by now
        if is_cut {
            return;
        }

        let mut top_start = max_y;

        // render middle
        for y in (middle_start)..=(max_y) {
            let in_hex = check_inside(&mut edges);
            if !in_hex && is_bottom_row {
                top_start = y;
                break;
            }
            img.put_hor_line((min_x as u32, max_x as u32 + 1), y as u32, color);
            for (index, edge) in edges.iter_mut().enumerate() {
                *edge -= deltas[index].0;
            }
        }

        // don't render bottom part, because it will be overwritten
        if !is_bottom_row {
            return
        }

        let mut left_border = min_x;
        let mut right_border = max_x;

        // render bottom
        for y in (top_start)..=(max_y) {
            let is_reversed = (((y - top_start) % 2) != 0) ^ middle_start_reversed;

            if is_reversed {
                for (x_index, x) in (left_border..=right_border).rev().enumerate() {
                    let in_hex = check_inside(&mut edges);
                    line_state.update(in_hex);
                    
                    if in_hex {
                        img.put_pixel(x as u32, y as u32, color);
                    } else {
                        // skip to the next line
                        match line_state {
                            LineState::AfterHex => {
                                left_border = x;
                                break
                            },
                            _ => {}
                        }
                    }

                    // dont add offset if the tested pixel is last on line
                    if x_index as i32 != (max_x - min_x) {
                        for (index, edge) in edges.iter_mut().enumerate() {
                            *edge -= deltas[index].1;
                        }
                    }
                }
            } else {
                for (x_index, x) in (left_border..=right_border).enumerate() {
                    let in_hex = check_inside(&mut edges);
                    line_state.update(in_hex);
                    
                    if in_hex {
                        img.put_pixel(x as u32, y as u32, color);
                    } else {
                        // skip to the next line
                        match line_state {
                            LineState::AfterHex => {
                                right_border = x;
                                break
                            },
                            _ => {}
                        }
                    }

                    // dont add offset if the tested pixel is last on line
                    if x_index as i32 != (max_x - min_x) {
                        for (index, edge) in edges.iter_mut().enumerate() {
                            *edge += deltas[index].1;
                        }
                    }
                }
            }

            line_state.reset();

            for (index, edge) in edges.iter_mut().enumerate() {
                *edge -= deltas[index].0;
            }
        }
    }

    fn render_hex(&self, image: &mut Image, hex: &Hex, width: u32, render_wrapped: RenderWrapped, is_bottom_row: bool) {
        let mut rng = thread_rng();
        // randomize color a little bit
        let color_diff = rng.gen_range(0.98, 1.02);

        // get hex vertices positions
        // points need to be in counter clockwise order
        let mut points = [(0.0, 0.0);6];
        for index in 0..6 {
            let coords = self.get_hex_vertex(hex, index);
            if self.antialiasing {
                points[5 - index] = (coords.0 * self.multiplier * SUPERSAMPLE_FACTOR as f32, coords.1 * self.multiplier * SUPERSAMPLE_FACTOR as f32);
            } else {
                points[5 - index] = (coords.0 * self.multiplier, coords.1 * self.multiplier);
            }
        };

        let clamp_color = |value: f32| {
            (value).max(0.0).min(255.0) as u8
        };

        let mut color = match hex.terrain_type {
            HexType::Debug(val) => {
                let value = clamp_color(val * 255.0);
                [value, value, value]
            },
            HexType::Debug2d((r,g)) => {
                [clamp_color(r * 255.0), clamp_color(g * 255.0), 0]
            },
            _ => {
                let color = self.colors.get_color_u8(&hex.terrain_type);
                [color.0, color.1, color.2]
            }
        };

        // dont't randomize color of debug hexes
        if self.randomize_colors {
            match hex.terrain_type {
                HexType::Debug(_) | HexType::Debug2d(_) => {},
                _ => {
                    for i in 0..3 {
                        color[i] = clamp_color(f32::from(color[i]) * color_diff);
                    }
                }
            }
        }

        self.render_hex_to_image(&points, image, color, is_bottom_row);

        let scale = if self.antialiasing {
            SUPERSAMPLE_FACTOR as f32
        } else {
            1.0
        };

        match render_wrapped {
            RenderWrapped::None => {},
            RenderWrapped::Left => {
                // subtract offset
                for index in 0..6 {
                    points[index].0 -= scale * width as f32 * self.multiplier;
                };
                self.render_hex_to_image(&points, image, color, is_bottom_row);
            },
            RenderWrapped::Right => {
                // add offset
                for index in 0..6 {
                    points[index].0 += scale * width as f32 * self.multiplier;
                };
                self.render_hex_to_image(&points, image, color, is_bottom_row);
            }
        };
    }

    fn render_aa_image(&self, map: &HexMap) -> Image {
        //let time = Instant::now();
        let width = (map.absolute_size_x * self.multiplier) as u32;
        let height = (map.absolute_size_y * self.multiplier) as u32;
        let mut image_final = Image::new(width, height, ColorMode::Rgb);

        let mut image_supersampled = Image::new(width * SUPERSAMPLE_FACTOR, height * SUPERSAMPLE_FACTOR, ColorMode::Rgb);

        for (index, hex) in map.field.iter().enumerate() {
            let wrapping = if self.wrap_map && index as u32 % map.size_x == 0 {
                RenderWrapped::Right
            } else if self.wrap_map && index as u32 % map.size_x == (map.size_x - 1) {
                RenderWrapped::Left
            } else {
                RenderWrapped::None
            };
            // check bottom row
            if hex.y as u32 == map.size_y - 1 {
                self.render_hex(&mut image_supersampled, hex, map.size_x, wrapping, true);
            } else {
                self.render_hex(&mut image_supersampled, hex, map.size_x, wrapping, false);
            }
        }

        // downsample
        for x in 0..width {
            for y in 0..height {
                let mut total_red = 0;
                let mut total_green = 0;
                let mut total_blue = 0;
                for i in 0..SUPERSAMPLE_FACTOR {
                    for j in 0..SUPERSAMPLE_FACTOR {
                        let pixel = image_supersampled.get_pixel((SUPERSAMPLE_FACTOR * x) + i, (SUPERSAMPLE_FACTOR * y) + j);
                        total_red+=pixel[0] as u32;
                        total_green+=pixel[1] as u32;
                        total_blue+=pixel[2] as u32;
                    }
                }
                let avg_red: u32 = total_red / (SUPERSAMPLE_FACTOR * SUPERSAMPLE_FACTOR);
                let avg_green: u32 = total_green / (SUPERSAMPLE_FACTOR * SUPERSAMPLE_FACTOR);
                let avg_blue: u32 = total_blue / (SUPERSAMPLE_FACTOR * SUPERSAMPLE_FACTOR);
                image_final.put_pixel(x as u32, y as u32, [avg_red as u8, avg_green as u8, avg_blue as u8]);
            }
        }

        image_final
    }

    pub fn set_random_colors(&mut self, value: bool) {
        self.randomize_colors = value;
    }

    pub fn use_antialiasing(&mut self, value: bool) {
        self.antialiasing = value;
    }
}

impl Default for Basic {
    fn default() -> Basic {
        Basic{multiplier: 50.0, wrap_map: true, randomize_colors: true, antialiasing: true, colors: ColorMap::new()}
    }
}

impl Renderer for Basic {
    fn render(&self, map: &HexMap) -> Image {
        if self.antialiasing {
            return self.render_aa_image(map);
        }

        let width = (map.absolute_size_x * self.multiplier) as u32;
        let height = (map.absolute_size_y * self.multiplier) as u32;
        let mut image = Image::new(width, height, ColorMode::Rgb);

        for (index, hex) in map.field.iter().enumerate() {
            let wrapping = if self.wrap_map && index as u32 % map.size_x == 0 {
                RenderWrapped::Right
            } else if self.wrap_map && index as u32 % map.size_x == (map.size_x - 1) {
                RenderWrapped::Left
            } else {
                RenderWrapped::None
            };
            // check bottom row
            if hex.y as u32 == map.size_y - 1 {
                self.render_hex(&mut image, hex, map.size_x, wrapping, true);
            } else {
                self.render_hex(&mut image, hex, map.size_x, wrapping, false);
            }
        }
        image
    }

    fn set_scale(&mut self, scale: f32) {
        if scale > 0.0 {
            self.multiplier = scale;
        } else {
            self.multiplier = 50.0;
            eprintln!("Tried to set negative scale, setting default scale instead.");
        }
    }

    fn set_wrap_map(&mut self, value: bool) {
        self.wrap_map = value;
    }
}

enum RenderWrapped {
    Left,
    Right,
    None
}

enum LineState {
    BeforeHex,
    InHex,
    AfterHex
}

impl LineState {
    pub fn reset(&mut self) {
        *self = LineState::BeforeHex;
    }

    pub fn update(&mut self, in_hex: bool) {
        *self = match self {
            LineState::BeforeHex if in_hex => LineState::InHex,
            LineState::InHex if !in_hex => LineState::AfterHex,
            _ => return
        };
    } 
}