

use std::str::FromStr;
use std::io::Read;
use std::io::StdinLock;

use rayon::prelude::*;

use super::action;
use super::board;
use super::player;
use super::rensa_plan;
use super::skill_plan;
use super::replay;
use super::consts::{W,H,MAX_TURN};

use super::rand;

struct Moves {
    actions: Vec<action::Action>,
    obstacles: Vec<i32>,
}

pub struct BestAi<'a> {
}

impl<'a> BestAi<'a> {
    pub fn exec(&mut self) {
        println!("test-best-ai");
        self.read_game_input();
        loop {
            self.read_turn_input();
            let act = self.think();
            println!("{}", act);
        }
    }

    fn think(&mut self) -> action::Action {
        let enemy_moves = self.search_enemy_moves();
        self.search(&enemy_moves)
    }

    fn search_enemy_moves(player: &player::Player) -> Vec<Moves> {
        ;
    }

    fn search(&self, player: &player::Player, enemy_moves: &[Moves]) -> action::Action {
        ;
    }
}
