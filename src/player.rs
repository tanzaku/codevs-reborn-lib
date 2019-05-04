
use super::action;
use super::board;

use super::consts::*;

#[derive(Clone, Default, PartialEq, Eq)]
pub struct Player {
    pub board: board::Board,
    pub obstacle: i32,
    pub skill_guage: i32,
    pub decrease_skill_guage: i32,
}

impl Player {
    pub fn new(board: board::Board, obstacle: i32, skill_guage: i32) -> Self {
        Self { board, obstacle, skill_guage, decrease_skill_guage: 0, }
    }

    pub fn put_one(&mut self, v: u64, pos: usize) -> action::ActionResult {
        if self.obstacle >= W as i32 {
            // self.obstacle -= W;
            // self.board.fall_obstacle();
            self.board.put_one(OBSTACLE, pos);
        }
        
        self.board.put_one(v, pos)
    }

    pub fn put(&mut self, pack: &[[u8; 2]; 2], action: &action::Action) -> action::ActionResult {
        if self.obstacle >= W as i32 {
            self.obstacle -= W as i32;
            self.board.fall_obstacle();
        }
        
        let result = match action {
            action::Action::PutBlock { pos, rot } => {
                let result = self.board.put(pack, *pos, *rot);
                if result.chains > 0 {
                    self.skill_guage += 8;
                }
                result
            },
            action::Action::UseSkill => {
                let result = self.board.use_skill();
                self.skill_guage = 0;
                result
            },
        };
        self.obstacle -= result.obstacle;
        self.decrease_skill_guage += result.skill_guage;
        result
    }

    pub fn add_obstacles(&mut self, obstacle: i32) {
        self.obstacle += obstacle;
    }

    pub fn can_use_skill(&self) -> bool {
        self.skill_guage >= 80
    }

    pub fn hash(&self) -> u64 {
        self.board.hash()
    }
}
