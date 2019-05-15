


use super::action;
use super::score_calculator;

use super::consts::{W,H,VANISH,OBSTACLE};

pub struct Feature {
    pub keima: i32,
    pub keima2: i32,
    pub tate: i32,
    pub tate2: i32,
    pub num_block: i32,
    pub var: i32,
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

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.column.iter().all(|b| *b == 0)
    }

    pub fn from_board(board: [u8; (W * H) as usize]) -> Self {
        let mut b = Board::new();
        for y in 0..H {
            for x in 0..W {
                b.column[x] |= (board[(H-1-y)*W+x] as u64) << (4 * y);
            }
        }
        b
    }

    #[inline]
    fn height(&self, x: usize) -> usize {
        ((64 - self.column[x].leading_zeros() + 3) / 4) as usize
    }

    #[inline]
    fn height_by_val(v: u64) -> u8 {
        ((64 - v.leading_zeros() + 3) / 4) as u8
    }

    #[inline]
    fn fall(&mut self, x: usize, v: u64) {
        let h = self.height(x);
        if h == 16 { self.dead = true; return; }
        self.column[x] ^= v << (h * 4);
    }

    #[inline]
    pub fn calc_max_rensa_by_erase_outer_block(&self) -> (Board, action::ActionResult, (usize, usize)) {
        let mut heights = [0; W];
        (0..W).for_each(|i| {
            heights[i] = self.height(i);
        });

        let vanish_result = (0..W).map(|x| {
            let l = {
                let mut l = H;
                if x > 0 { l = std::cmp::min(l, heights[x-1]); }
                if x + 1 < W { l = std::cmp::min(l, heights[x+1]); }
                std::cmp::max(l, 1) - 1
            };
            let h = std::cmp::max(heights[x], 1) - 1;

            let r = (l..h).map(|y| {
                if (self.column[x] >> (y*4) & 0x0F) == OBSTACLE {
                    return Default::default();
                }

                let mut b = self.clone();
                unsafe {
                    use std::arch::x86_64::*;
                    b.column[x] = _pext_u64(b.column[x], !(0x0F << (y*4)));
                }
                let changed = 1<<x;
                let r = b.vanish(changed);
                (b, r, (x, y))
            }).max_by_key(|r| (r.1).0);
            r
        }).filter(|r| r.is_some()).map(|r| r.unwrap()).max_by_key(|r| (r.1).0);

        let (board, vanish_result, p) = vanish_result.unwrap_or(Default::default());
        // (board, score_calculator::ScoreCalculator::calc_chain_result(vanish_result.0, vanish_result.1), p)
        (board, score_calculator::ScoreCalculator::calc_chain_result(vanish_result.0, 0), p)
    }

    #[inline]
    pub fn calc_max_rensa_by_erase_block_over_obstacle(&self) -> (Board, action::ActionResult, (usize, usize)) {
        let mut heights = [0; W];
        let mut highest_obstacle_row = [0; W];
        (0..W).for_each(|i| {
            highest_obstacle_row[i] = ((64 - Self::calc_obstacle_mask(self.column[i]).leading_zeros()) / 4) as usize;
            heights[i] = self.height(i);
        });

        let vanish_result = (0..W).map(|x| {
            let l = {
                let mut l = H;
                if x > 0 { l = std::cmp::min(l, highest_obstacle_row[x-1]); }
                if x + 1 < W { l = std::cmp::min(l, highest_obstacle_row[x+1]); }
                std::cmp::max(l, 1) - 1
            };
            let h = std::cmp::max(heights[x], 1) - 1;

            let r = (l..h).map(|y| {
                if (self.column[x] >> (y*4) & 0x0F) == OBSTACLE {
                    return Default::default();
                }

                let mut b = self.clone();
                unsafe {
                    use std::arch::x86_64::*;
                    b.column[x] = _pext_u64(b.column[x], !(0x0F << (y*4)));
                }
                let changed = 1<<x;
                let r = b.vanish(changed);
                (b, r, (x, y))
            }).max_by_key(|r| (r.1).0);
            r
        }).filter(|r| r.is_some()).map(|r| r.unwrap()).max_by_key(|r| (r.1).0);

        let (board, vanish_result, p) = vanish_result.unwrap_or(Default::default());
        // (board, score_calculator::ScoreCalculator::calc_chain_result(vanish_result.0, vanish_result.1), p)
        (board, score_calculator::ScoreCalculator::calc_chain_result(vanish_result.0, 0), p)
    }

    #[inline]
    pub fn calc_max_rensa_by_erase_block(&self) -> (Board, action::ActionResult, (usize, usize)) {
        self.calc_max_rensa_by_erase_block_over_obstacle()
        // self.calc_max_rensa_by_erase_outer_block()
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
        score_calculator::ScoreCalculator::calc_chain_result(vanish_result.0, vanish_result.1)
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
            bombed_block += vanished[x].count_ones();
        });

        let changed = self.fall_by_mask(&vanished);
        let vanish_result = self.vanish(changed);
        score_calculator::ScoreCalculator::calc_bomb_result(bombed_block as u8, vanish_result.0, vanish_result.1)
    }

    pub fn calc_feature(&self) -> Feature {
        let mut keima = 0;
        let mut keima2 = 0;
        let mut tate = 0;
        let mut tate2 = 0;
        let mut var = 0;
        let mut heights = [0; W];
        (0..W).for_each(|i| heights[i] = self.height(i));
        for i in 0..W-1 {
            let r = Self::calc_remove(self.column[i], self.column[i]<<8);
            tate += r.count_ones();
            
            let r = Self::calc_remove(self.column[i], self.column[i]<<12);
            tate2 += r.count_ones();

            let r = Self::calc_remove(self.column[i], self.column[i+1]<<8);
            keima += r.count_ones();
            
            let r = Self::calc_remove(self.column[i], self.column[i+1]>>8);
            keima += r.count_ones();
            
            let r = Self::calc_remove(self.column[i], self.column[i+1]<<12);
            keima2 += r.count_ones();
            
            let r = Self::calc_remove(self.column[i], self.column[i+1]>>12);
            keima2 += r.count_ones();

            var += (heights[i] - heights[i+1]) * (heights[i] - heights[i+1]);
        }
        let r = Self::calc_remove(self.column[W-1], self.column[W-1]<<8);
        tate += r.count_ones();

        let r = Self::calc_remove(self.column[W-1], self.column[W-1]<<12);
        tate2 += r.count_ones();
        
        let num_block = (0..W).map(|x| self.height(x) as i32).sum();

        Feature {
            keima: keima as i32,
            keima2: keima2 as i32,
            tate: tate as i32,
            tate2: tate2 as i32,
            num_block,
            var: var as i32,
        }
    }

    #[inline]
    fn calc_five_mask(c: u64) -> u64 {
        // 5 -> 0101
        let mask = 0x1111111111111111;
        let d = !c;
        let v = c & (d >> 1) & (c >> 2) & (d >> 3) & mask;
        v
    }

    #[inline]
    fn calc_obstacle_mask(c: u64) -> u64 {
        // 11 -> 1011
        let mask = 0x1111111111111111;
        let d = !c;
        let v = c & (c >> 1) & (d >> 2) & (c >> 3) & mask;
        v
    }

    #[inline]
    fn calc_empty_mask(c: u64) -> u64 {
        // 0 -> 0000
        let mask = 0x1111111111111111;
        let d = !c;
        let v = d & (d >> 1) & (d >> 2) & (d >> 3) & mask;
        v
    }

    // #[inline]
    // fn calc_remove0(c1: u64, c2: u64) -> u64 {
    //     let mask = 0x0101010101010101;
    //     let c = c1 + c2;
    //     let d = !c;
    //     let v = d & (c >> 1) & (d >> 2) & (c >> 3) & (d >> 4) & mask;
    //     v
    // }

    // /**
    //  * 足して10になる位置のビットのみ1が立っている
    //  */
    // #[inline]
    // fn calc_remove(c1: u64, c2: u64) -> u64 {
    //     let mask = 0x0F0F0F0F0F0F0F0F;
    //     let v1 = Self::calc_remove0(c1 & mask, c2 & mask);
    //     let v2 = Self::calc_remove0(c1 >> 4 & mask, c2 >> 4 & mask) << 4;
    //     v1 ^ v2
    // }

    #[inline]
    fn calc_remove0(c1: u64, c2: u64) -> u64 {
        let mask = 0x0101010101010101;
        let c = c1 + c2;
        let d = !c;
        let v = d & (c >> 1) & (d >> 2) & (c >> 3) & (d >> 4) & mask;
        v
    }

    /**
     * 足して10になる位置のビットのみ1が立っている
     */
    #[inline]
    fn calc_remove_ref(c1: u64, c2: u64) -> u64 {
        let mask = 0x0F0F0F0F0F0F0F0F;
        let v1 = Self::calc_remove0(c1 & mask, c2 & mask);
        let v2 = Self::calc_remove0(c1 >> 4 & mask, c2 >> 4 & mask) << 4;
        v1 ^ v2
    }

    /**
     * 足して10になる位置のビットのみ1が立っている
     */
    #[inline]
    fn calc_remove(c1: u64, c2: u64) -> u64 {
        // let res_ref = Self::calc_remove_ref(c1, c2);

        let mask1 = 0x1111111111111111;
        let mask8 = 0x8888888888888888;
        let lsb1 = c1 & mask1;
        let lsb2 = c2 & mask1;
        let c1 = (c1 >> 1) & !mask8;
        let c2 = (c2 >> 1) & !mask8;

        // c = 0101, lsb1 ^ lsb2 = 0
        let c = c1 + c2 + (lsb1 & lsb2);
        let c = ((!c) >> 1) & c;
        let res = !(lsb1 ^ lsb2) & c & (c >> 2) & mask1;
        // assert_eq!(res_ref, res);
        res
    }

    #[inline]
    fn fall_by_mask(&mut self, mask: &[u64]) -> usize {
        let mut changed = 0;
        for i in 0..mask.len() {
            if mask[i] != 0 {
                changed |= 1 << i;
            }
            unsafe {
                use std::arch::x86_64::*;
                self.column[i] = _pext_u64(self.column[i], !(mask[i] * 0x0F));
            }
        }
        changed
    }

    fn vanish(&mut self, changed: usize) -> (u8, i8) {
        let mut rensa = 0;
        let mut changed = changed;
        let mut height = 111;

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
            }
            let r = Self::calc_remove(self.column[W-1], self.column[W-1]<<4);
            remove_mask[W-1] |= r;
            remove_mask[W-1] |= r >> 4;

            // eprintln!("{:?}", self);
            if height == 111 {
                let mut not_changed_max = 0;
                let mut changed_max = 0;
                remove_mask.iter().enumerate().for_each(|(x,m)| {
                    if (changed >> x & 1) == 0 {
                        not_changed_max = std::cmp::max(not_changed_max, Self::height_by_val(*m));
                    } else {
                        changed_max = std::cmp::max(changed_max, Self::height_by_val(*m));
                    }
                });
                height = (not_changed_max as i8) - (changed_max as i8);
            }
            changed = self.fall_by_mask(&remove_mask);
            if changed == 0 {
                break;
            }
            rensa += 1;
        }
        (rensa, height)
    }

    #[inline]
    pub fn fall_obstacle(&mut self) {
        for x in 0..W {
            self.fall(x, OBSTACLE);
        }
    }

    #[inline]
    pub fn is_dead(&self) -> bool {
        self.dead
    }

    #[inline]
    pub fn adjust_height_min(&self, x: usize) -> usize {
        let mut h = H;
        if x > 0 { h = std::cmp::min(h, self.height(x-1)); }
        if x < W - 1 { h = std::cmp::min(h, self.height(x+1)); }
        h
    }

    #[inline]
    pub fn adjust_height_max(&self, x: usize) -> usize {
        let mut h = 0;
        if x > 0 { h = std::cmp::max(h, self.height(x-1)); }
        if x < W - 1 { h = std::cmp::max(h, self.height(x+1)); }
        h
    }

    #[inline]
    pub fn max_height(&self) -> usize {
        (0..W).map(|x| self.height(x)).max().unwrap()
    }

    #[inline]
    pub fn num_obstacle(&self) -> u64 {
        self.column.iter().map(|c| Self::calc_obstacle_mask(*c)).sum::<u64>()
    }

    #[inline]
    pub fn hash(&self) -> u64 {
        let mut h = 0;
        self.column.iter().for_each(|c| h = h*31+c);
        h
    }
}

impl std::fmt::Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "dump board").expect("");
        writeln!(f, "{:?}", self.column).expect("");
        for y in (0..H).rev() {
            // let mut res = String::new();
            self.column.iter().for_each(|c| {
                let c = c >> (y * 4) & 0xF;
                let c = if c > VANISH { 'X' } else { std::char::from_digit(c as u32, 10).unwrap() };
                // res += &c.to_string();
                write!(f, "{}", c).expect("");
            });
            writeln!(f, "").expect("");
        }
        Ok(())
    }
}

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
    assert_eq!(board.column, [27, 136, 0, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn board_test_2() {
    let mut board = Board::new();
    board.column[0] = 0x17B1819;
    board.column[1] = 0x0098832;
    board.put(&[[9,5],[0,3]], 1, 3);
    assert_eq!(board.column, [11, 1416, 3, 0, 0, 0, 0, 0, 0, 0]);
}
