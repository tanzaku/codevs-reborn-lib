


use std::ops::{Index,IndexMut};
use std::collections::HashSet;

use super::action;
use super::score_calculator;

use super::consts::{W,H,VANISH};

#[derive(Clone)]
pub struct Board {
    column: [u64; W],
}

// fn rotate(pattern: &[[u8; 2]; 2], rot: usize) -> [[u8; 2]; 2] {
//     let mut rot = rot;
//     let mut pattern = pattern.clone();
//     while rot > 0 {
//         pattern = [[pattern[1][0], pattern[0][0]],
//                     [pattern[1][1], pattern[0][1]]];
//         rot -= 1;
//     }
//     pattern
// }

impl Board {
    pub fn new() -> Self {
        Self {
            column: [0; W],
        }
    }

    pub fn from_board(board: [u8; (W * H) as usize]) -> Self {
        unimplemented!()
    }

    fn top_empty_pos(&self, x: usize) -> usize {
        // self.height[x] as usize * W as usize + x
        unimplemented!()
    }

    pub fn put(&mut self, pattern: &[[u8; 2]; 2], pos: usize, rot: usize) -> action::ActionResult {
        // assert!(pos + pattern.len() <= W as usize);
        // let pattern = rotate(pattern, rot);
        // let mut dh = [0; 2];
        // for dx in 0..pattern[0].len() {
        //     let x = pos + dx;
        //     for dy in (0..pattern.len()).rev() {
        //         if pattern[dy][dx] != 0 {
        //             let p = self.top_empty_pos(x);
        //             self.board[p] = pattern[dy][dx];
        //             self.height[x] += 1;
        //             dh[dx] += 1;
        //         }
        //     }
        // }

        // fixed changed
        let chains = self.vanish(1 << pos | 1 << (pos+1));
        score_calculator::ScoreCalculator::calc_chain_result(chains)
        // unimplemented!()
    }

    pub fn use_skill(&mut self) -> action::ActionResult {
        // let mut set = HashSet::new();
        // for x in 0..W-1 {
        //     for y in 0..self.height[x as usize] as i32 {
        //         let p = y * W + x;
        //         if self[p] != 5 {
        //             continue;
        //         }
        //         DIRS9.into_iter().filter(|d| self[p+*d] < VANISH).for_each(|d| { set.insert(p + d); });
        //     }
        // }
        // if set.is_empty() {
        //     return action::ActionResult::new(0, 0, 0);
        // }
        // set.iter().for_each(|p| { self[*p] = 0; });
        // self.fall_down();
        // let bombed_block = set.len() as u8;
        // let chains = self.vanish();
        // score_calculator::ScoreCalculator::calc_bomb_result(bombed_block, chains)
        unimplemented!()
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

    fn vanish(&mut self, changed: usize) -> u8 {
        let mut rensa = 0;
        let mut changed = changed;
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

            eprintln!("{:?}", self);
            changed = 0;
            for i in 0..remove_mask.len() {
                if remove_mask[i] != 0 {
                    changed |= 1 << i;
                }
                unsafe {
                    use std::arch::x86_64::*;
                    self.column[i] = _pext_u64(self.column[i], !remove_mask[i]);
                }
            }
            if changed == 0 {
                break;
            }
            rensa += 1;
        }
        rensa
    }

    pub fn fall_obstacle(&mut self) {
        // if self.is_dead() {
        //     return;
        // }
        // for x in 0..W-1 {
        //     let y = self.height[x as usize] as i32;
        //     self[(x,y)] = OBSTACLE;
        //     self.height[x as usize] += 1;
        // }
        unimplemented!()
    }

    pub fn is_dead(&self) -> bool {
        // *self.height.iter().max().unwrap() as i32 > DEAD_LINE_Y
        unimplemented!()
    }

    pub fn max_height(&self) -> u8 {
        // *self.height.iter().max().unwrap()
        unimplemented!()
    }
}

impl std::fmt::Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // write!(f, "Point {{ x: {}, y: {} }}", self.x, self.y)
        let mut res = String::new();
        for y in (0..H).rev() {
            self.column.iter().for_each(|c| {
                let c = c >> (y * 4) & 0xF;
                let c = if c > VANISH { 'X' } else { std::char::from_digit(c as u32, 10).unwrap() };
                res += &c.to_string();
            });
            res += "\n";
        }
        write!(f, "{}", res)
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
fn board_test() {
    let mut board = Board::new();
    board.column[0] = 0x17B1819;
    board.column[1] = 0x0098832;
    // let rensa = board.put(&[[9,5],[0,3]], 8, 3);
    let rensa = board.put(&[[9,5],[0,3]], 1, 3);
    eprintln!("{:?}", board);
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
