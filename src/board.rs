


use std::ops::{Index,IndexMut};
use std::collections::HashSet;

use super::action;
use super::score_calculator;

use super::consts::{W,H,VANISH,OBSTACLE};

pub struct Feature {
    pub keima: i32,
    pub keima2: i32,
    pub tate: i32,
    pub tate2: i32,
}

#[derive(Clone)]
pub struct Board {
    column: [u64; W],
    dead: bool,
}

fn rotate(pattern: &[[u8; 2]; 2], rot: usize) -> [[u8; 2]; 2] {
    let mut rot = rot;
    let mut pattern = pattern.clone();
    while rot > 0 {
        pattern = [[pattern[1][0], pattern[0][0]],
                    [pattern[1][1], pattern[0][1]]];
        rot -= 1;
    }
    pattern
}

impl Board {
    pub fn new() -> Self {
        Self {
            column: [0; W],
            dead: false,
        }
    }

    pub fn from_board(board: [u8; (W * H) as usize]) -> Self {
        let mut b = Board::new();
        for y in 0..H {
            for x in 0..W {
                b.column[x] |= (board[(H-1-y)*W+x] as u64) << (4 * y);
            }
        }
        // eprintln!("{:?}", b);
        b
    }

    fn height(&self, x: usize) -> usize {
        ((64 - self.column[x].leading_zeros() + 3) / 4) as usize
    }

    fn height_by_val(v: u64) -> u8 {
        ((64 - v.leading_zeros() + 3) / 4) as u8
    }

    fn fall(&mut self, x: usize, v: u64) {
        let h = self.height(x);
        if h == 16 { self.dead = true; return; }
        self.column[x] ^= v << (h * 4);
    }

    pub fn put_one(&mut self, v: u64, pos: usize) -> action::ActionResult {
        self.fall(pos, v);
        let changed = 1 << pos;
        let vanish_result = self.vanish(changed);
        score_calculator::ScoreCalculator::calc_chain_result(vanish_result.0, vanish_result.1, vanish_result.2)
    }

    pub fn put(&mut self, pattern: &[[u8; 2]; 2], pos: usize, rot: usize) -> action::ActionResult {
        let mut changed = 0;
        let pattern = rotate(pattern, rot);
        (0..2).for_each(|d| {
            pattern.iter().rev().for_each(|p| {
                if p[d] == 0 { return; }
                self.fall(pos + d, p[d].into());
                changed |= 1 << (pos + d);
            });
        });

        // fixed changed
        let vanish_result = self.vanish(changed);
        score_calculator::ScoreCalculator::calc_chain_result(vanish_result.0, vanish_result.1, vanish_result.2)
    }

    pub fn use_skill(&mut self) -> action::ActionResult {
        let mut vanished = [0; W];

        (0..W).for_each(|x| {
            let fives = Self::calc_five_mask(self.column[x]);
            let bombed_mask = fives << 4 | fives | fives >> 4;
            vanished[x] |= bombed_mask;
            if x > 0 { vanished[x-1] |= bombed_mask; }
            if x < W - 1 { vanished[x+1] |= bombed_mask; }
        });

        let mut bombed_block = 0;
        (0..W).for_each(|x| {
            let obstacle_mask = Self::calc_obstacle_mask(self.column[x]);
            let empty_mask = Self::calc_empty_mask(self.column[x]);
            vanished[x] &= !obstacle_mask;
            vanished[x] &= !empty_mask;
            bombed_block += vanished[x].count_ones() / 4;   // 4bit maskなので4で割る
        });

        let changed = self.fall_by_mask(&vanished);
        let vanish_result = self.vanish(changed);
        score_calculator::ScoreCalculator::calc_bomb_result(bombed_block as u8, vanish_result.0, vanish_result.1, vanish_result.2)
    }

    pub fn calc_feature(&self) -> Feature {
        let mut keima = 0;
        let mut keima2 = 0;
        let mut tate = 0;
        let mut tate2 = 0;
        for i in 0..W-1 {
            let r = Self::calc_remove(self.column[i], self.column[i]<<8);
            tate += r.count_ones() / 4;
            
            let r = Self::calc_remove(self.column[i], self.column[i]<<12);
            tate2 += r.count_ones() / 4;

            let r = Self::calc_remove(self.column[i], self.column[i+1]<<8);
            keima += r.count_ones() / 4;
            
            let r = Self::calc_remove(self.column[i], self.column[i+1]>>8);
            keima += r.count_ones() / 4;
            
            let r = Self::calc_remove(self.column[i], self.column[i+1]<<12);
            keima2 += r.count_ones() / 4;
            
            let r = Self::calc_remove(self.column[i], self.column[i+1]>>12);
            keima2 += r.count_ones() / 4;
        }
        let r = Self::calc_remove(self.column[W-1], self.column[W-1]<<8);
        tate += r.count_ones() / 4;

        let r = Self::calc_remove(self.column[W-1], self.column[W-1]<<12);
        tate2 += r.count_ones() / 4;
        
        Feature {
            keima: keima as i32,
            keima2: keima2 as i32,
            tate: tate as i32,
            tate2: tate2 as i32,
        }
    }

    fn calc_five_mask(c: u64) -> u64 {
        // 5 -> 0101
        let mask = 0x1111111111111111;
        let d = !c;
        let v = c & (d >> 1) & (c >> 2) & (d >> 3) & mask;
        v * 0x0F
    }

    fn calc_obstacle_mask(c: u64) -> u64 {
        // 11 -> 1011
        let mask = 0x1111111111111111;
        let d = !c;
        let v = c & (c >> 1) & (d >> 2) & (c >> 3) & mask;
        v * 0x0F
    }

    fn calc_empty_mask(c: u64) -> u64 {
        // 0 -> 0000
        let mask = 0x1111111111111111;
        let d = !c;
        let v = d & (d >> 1) & (d >> 2) & (d >> 3) & mask;
        v * 0x0F
    }

    fn calc_remove0(c1: u64, c2: u64) -> u64 {
        let mask = 0x0101010101010101;
        let c = c1 + c2;
        let d = !c;
        let v = d & (c >> 1) & (d >> 2) & (c >> 3) & (d >> 4) & mask;
        v * 0x0F
    }

    /**
     * 足して10になる位置のビットのみ1が立っている
     */
    fn calc_remove(c1: u64, c2: u64) -> u64 {
        let mask = 0x0F0F0F0F0F0F0F0F;
        let v1 = Self::calc_remove0(c1 & mask, c2 & mask);
        let v2 = Self::calc_remove0(c1 >> 4 & mask, c2 >> 4 & mask) << 4;
        v1 ^ v2
    }

    fn fall_by_mask(&mut self, mask: &[u64]) -> usize {
        let mut changed = 0;
        for i in 0..mask.len() {
            if mask[i] != 0 {
                changed |= 1 << i;
            }
            unsafe {
                use std::arch::x86_64::*;
                self.column[i] = _pext_u64(self.column[i], !mask[i]);
            }
        }
        changed
    }

    fn vanish(&mut self, changed: usize) -> (u8, u8, u64) {
        let mut rensa = 0;
        let mut changed = changed;
        let mut height = 0;
        // let mut remove_hash = 0;

        loop {
            let c = changed | changed >> 1;
            let mut remove_mask = [0; W];

            for i in 0..W-1 {
                if (c & (1<<i)) == 0 {
                    continue
                }
                
                let r = Self::calc_remove(self.column[i], self.column[i]<<4);
                remove_mask[i+0] |= r;
                remove_mask[i+0] |= r >> 4;
                
                let r = Self::calc_remove(self.column[i], self.column[i+1]);
                remove_mask[i+0] |= r;
                remove_mask[i+1] |= r;
                
                let r = Self::calc_remove(self.column[i], self.column[i+1]<<4);
                remove_mask[i+0] |= r;
                remove_mask[i+1] |= r >> 4;
                
                let r = Self::calc_remove(self.column[i], self.column[i+1]>>4);
                remove_mask[i+0] |= r;
                remove_mask[i+1] |= r << 4;

                // remove_hash = remove_hash * 31 + remove_mask[i+0];
            }
            let r = Self::calc_remove(self.column[W-1], self.column[W-1]<<4);
            remove_mask[W-1] |= r;
            remove_mask[W-1] |= r >> 4;
            // remove_hash = remove_hash * 31 + remove_mask[W-1];

            // eprintln!("{:?}", self);
            if height == 0 {
                height = Self::height_by_val(*remove_mask.iter().max().unwrap());
            }
            changed = self.fall_by_mask(&remove_mask);
            if changed == 0 {
                break;
            }
            rensa += 1;
        }
        // (rensa, height, remove_hash)
        (rensa, height, 0)
    }

    pub fn fall_obstacle(&mut self) {
        for x in 0..W {
            self.fall(x, OBSTACLE);
        }
    }

    pub fn is_dead(&self) -> bool {
        self.dead
    }

    pub fn max_height(&self) -> usize {
        (0..W).map(|x| self.height(x)).max().unwrap()
    }

    pub fn hash(&self) -> u64 {
        let mut h = 0;
        self.column.iter().for_each(|c| h += h * 31 + c);
        h
    }
}

impl std::fmt::Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let res = writeln!(f, "dump board");
        writeln!(f, "{:?}", self.column);
        for y in (0..H).rev() {
            // let mut res = String::new();
            self.column.iter().for_each(|c| {
                let c = c >> (y * 4) & 0xF;
                let c = if c > VANISH { 'X' } else { std::char::from_digit(c as u32, 10).unwrap() };
                // res += &c.to_string();
                write!(f, "{}", c);
            });
            writeln!(f, "");
        }
        res
    }
}

// impl Index<i32> for Board {
//     type Output = u8;

//     fn index(&self, ix: i32) -> &Self::Output {
//         // if ix as usize >= self.board.len() {
//         //     eprintln!("{:?}", self);
//         //     eprintln!("max height: {}", self.height.iter().max().unwrap());
//         // }
//         if ix < 0 || ix as usize >= self.board.len() {
//             return &OBSTACLE;
//         }
//         &self.board[ix as usize]
//     }
// }


// impl IndexMut<i32> for Board {
//     fn index_mut(&mut self, ix: i32) -> &mut u8 {
//         &mut self.board[ix as usize]
//     }
// }

// impl Index<(i32,i32)> for Board {
//     type Output = u8;

//     fn index(&self, ix: (i32,i32)) -> &Self::Output {
//         &self.board[(ix.1*W+ix.0) as usize]
//     }
// }

// impl IndexMut<(i32,i32)> for Board {
//     fn index_mut(&mut self, ix: (i32,i32)) -> &mut u8 {
//         &mut self.board[(ix.1*W+ix.0) as usize]
//     }
// }

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for Board {
    fn eq(&self, other: &Self) -> bool {
        self.column[..] == other.column[..]
    }
}

impl Eq for Board {}

#[test]
fn board_test_1() {
    let mut board = Board::new();
    board.column[0] = 0x07B1819;
    board.column[1] = 0x0008832;
    board.put(&[[1,9],[0,0]], 0, 0);
    // eprintln!("{:?}", board);
    assert_eq!(board.column, [27, 136, 0, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn board_test_2() {
    let mut board = Board::new();
    board.column[0] = 0x17B1819;
    board.column[1] = 0x0098832;
    board.put(&[[9,5],[0,3]], 1, 3);
    // eprintln!("{:?}", board);
    assert_eq!(board.column, [11, 1416, 3, 0, 0, 0, 0, 0, 0, 0]);
}

// #[test]
// fn board_test() {
//     let mut board = Board::new();
//     let rensa = board.put(&[[9,5],[0,3]], 8, 3);
//     // eprintln!("{:?}", board);
//     assert_eq!(board[(8,0)], 9);
//     assert_eq!(board[(9,0)], 3);
//     assert_eq!(board[(8,1)], 5);
//     assert_eq!(board[(9,1)], 0);
//     assert_eq!(rensa.obstacle, 0);
//     assert_eq!(rensa.skill_guage, 0);
// }

// #[test]
// fn board_test_vanish() {
//     let mut board = Board::new();
//     let rensa = board.put(&[[9,5],[0,1]], 8, 3);
//     // eprintln!("{:?}", board);
//     assert_eq!(board[(8,0)], 5);
//     assert_eq!(board[(9,0)], 0);
//     assert_eq!(board[(8,1)], 0);
//     assert_eq!(board[(9,1)], 0);
//     assert_eq!(rensa.obstacle, 0);
//     assert_eq!(rensa.skill_guage, 0);
// }

// #[test]
// fn board_test_vanish2() {
//     let mut board = Board::new();
//     let rensa = board.put(&[[2,0],[5,2]], 1, 0);
//     assert_eq!(board.max_height(), 2);
//     let rensa = board.put(&[[1,0],[5,6]], 1, 0);
//     assert_eq!(board.max_height(), 4);
// }
