use rand::{
    distributions::{Distribution, Standard},
    Rng
};
use crate::hexmap::HexMap;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
/// Data structure for single map tile
pub struct Hex {
    pub x: i32,
    pub y: i32,
    pub terrain_type: HexType,
    pub center_x: f32,
    pub center_y: f32
}

/// This is roughly ratio of hexagon height to width
pub const RATIO: f32 = 1.1547;

impl Hex {
    /// Creates new empty 'Hex' with default values
    pub fn new() -> Hex {
        Hex{..Default::default()}
    }

    /// Creates new `Hex` from specific coordinates with default `terrain_type`
    pub fn from_coords(x: i32, y: i32) -> Hex {
        let center_x = (x as f32) + (y/2) as f32 + match y % 2 {
            0 => 0.5,
            1 | _ => 1.0,
        };
        let center_y =  (y as f32 * RATIO * 3.0 / 4.0) + RATIO / 2.0;
        Hex{x, y, terrain_type: HexType::Water, center_x, center_y}    
    }

    /// Returns dstance to other `Hex`
    pub fn distance_to(&self, other: &Hex) -> i32 {
        ((self.x - other.x).abs() + (self.x + self.y - other.x - other.y).abs() + (self.y - other.y).abs()) / 2
    }

    /// Returns vector of `Hex` tiles next to specified `Hex`
    pub fn get_neighbours(&self, hexmap: &HexMap) -> Vec<(i32, i32)> {
        let mut neighbours: Vec<(i32, i32)> = Vec::with_capacity(6);

        if self.y != (hexmap.size_y as i32 - 1) {
            // bottom right
            neighbours.push(Hex::unwrap_coords(self.x, self.y + 1, hexmap.size_x));
            // bottom left
            neighbours.push(Hex::unwrap_coords(self.x - 1, self.y + 1, hexmap.size_x));
        }
        // left
        neighbours.push(Hex::unwrap_coords(self.x - 1, self.y, hexmap.size_x));

        if self.y != 0 {
            // top left
            neighbours.push(Hex::unwrap_coords(self.x, self.y - 1, hexmap.size_x));
            // top right
            neighbours.push(Hex::unwrap_coords(self.x + 1, self.y - 1, hexmap.size_x));
        }
        // right
        neighbours.push(Hex::unwrap_coords(self.x + 1, self.y, hexmap.size_x));

        neighbours
    }

        /// Returns vector of `Hex` tiles next to specified `Hex` without checking if contained in hexmap
    pub fn get_neighbours_unchecked(&self, hexmap: &HexMap) -> Vec<(i32, i32)> {
        let mut neighbours: Vec<(i32, i32)> = Vec::with_capacity(6);
        // bottom right
        neighbours.push(Hex::unwrap_coords(self.x, self.y + 1, hexmap.size_x));
        // bottom left
        neighbours.push(Hex::unwrap_coords(self.x - 1, self.y + 1, hexmap.size_x));
        // left
        neighbours.push(Hex::unwrap_coords(self.x - 1, self.y, hexmap.size_x));
        // top left
        neighbours.push(Hex::unwrap_coords(self.x, self.y - 1, hexmap.size_x));
        // top right
        neighbours.push(Hex::unwrap_coords(self.x + 1, self.y - 1, hexmap.size_x));
        // right
        neighbours.push(Hex::unwrap_coords(self.x + 1, self.y, hexmap.size_x));

        neighbours
    }

    /// Fixes coordinates which are out of bounds 
    pub fn unwrap_coords(x: i32, y: i32, size_x: u32) -> (i32, i32) {
        let mut new_x = x;
        if x < -(y/2) {
            new_x = x + size_x as i32;
        } else if x >= (size_x as i32 - y/2) {
            new_x = x - size_x as i32;
        }
        (new_x, y)
    }

    /// Returns ring of given radius around specified hex
    pub fn get_ring(&self, hexmap: &HexMap, radius: u32) -> Vec<(i32, i32)> {
        if radius == 0 {
            return vec!();
        }
        let mut results: Vec<(i32, i32)> = vec!();
        let mut hex = Hex::from_coords(self.x + radius as i32, self.y - radius as i32);
        for i in 0..6 {
            for _j in 0..radius {
                results.push((hex.x, hex.y));
                let neighbour = hex.get_neighbours_unchecked(hexmap)[i];
                hex = Hex::from_coords(neighbour.0, neighbour.1);
            }
        }
        results.iter().filter(|&s| {s.1 >= 0 && s.1 < hexmap.size_y as i32}).map(|&s| (s.0, s.1)).collect()
    }

    /// Returns spiral of given radius around specified hex
    pub fn get_spiral(&self, hexmap: &HexMap, radius: u32) -> Vec<(i32, i32)> {
        if radius == 0 {
            return vec!();
        }
        let mut results: Vec<Vec<(i32, i32)>> = vec!();
        results.push(vec!((self.x, self.y)));
        for i in 1..=radius {
            results.push(self.get_ring(hexmap, i));
        }
        results.iter().flatten().map(|&s| (s.0, s.1)).collect()
    }
}

 impl Default for Hex {
    fn default() -> Self {
        Hex{x:0, y: 0, terrain_type: HexType::Water, center_x: 0.5, center_y: RATIO / 2.0}
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Copy)]
/// Type of terrain / feature on specific 'Hex'
pub enum HexType {
    Field,
    Forest,
    Desert,
    Tundra,
    Water,
    Ocean,
    Mountain,
    Impassable,
    Ice,
    Jungle,
    Debug(f32),
    Debug2d((f32,f32)),
}

impl Distribution<HexType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> HexType {
        match rng.gen_range(0, 9) {
            0 => HexType::Field,
            1 => HexType::Forest,
            2 => HexType::Desert,
            3 => HexType::Tundra,
            4 => HexType::Water,
            5 => HexType::Ocean,
            6 => HexType::Mountain,
            7 => HexType::Impassable,
            8 => HexType::Ice,
            _ => HexType::Jungle
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex() {
        assert_eq!((9,1), Hex::unwrap_coords(-1, 1, 10));
        assert_eq!((-1,2), Hex::unwrap_coords(-1, 2, 10));
        assert_eq!((0,1), Hex::unwrap_coords(0, 1, 10));
        assert_eq!((0,1), Hex::unwrap_coords(10, 1, 10));
    }
}