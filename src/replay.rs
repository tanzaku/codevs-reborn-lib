

use std::collections::VecDeque;

use super::action;
use super::board;
use super::player;
use super::rensa_plan;
use super::skill_plan;
use super::consts::{W,H,MAX_TURN};


#[derive(Clone)]
pub struct Replay {
    expected_results: VecDeque<action::ActionResult>,
    packs: VecDeque<[[u8; 2]; 2]>,
    actions: VecDeque<u8>,
}

impl Replay {
    pub fn new() -> Self {
        Self {
            expected_results: VecDeque::new(),
            packs: VecDeque::new(),
            actions: VecDeque::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn can_replay(&self, player: &player::Player, enemy_send_obstacles: &[i32]) -> bool {
        if self.actions.is_empty() {
            return false;
        }

        let mut p = player.clone();
        let mut illegal_action = false;
        let mut turn = 0;
        let result = self.actions.iter().zip(self.packs.iter()).map(|(a, pack)| {
            let a = a.into();
            if a == action::Action::UseSkill && !p.can_use_skill() {
                illegal_action = true;
            }
            let result = p.put(pack, &a);
            if turn < enemy_send_obstacles.len() {
                p.add_obstacles(enemy_send_obstacles[turn]);
            }
            turn += 1;
            result
        }).last().unwrap();

        !illegal_action && &result == self.expected_results.back().unwrap()
    }

    pub fn init(&mut self, player: &player::Player, packs: &[[[u8; 2]; 2]], enemy_send_obstacles: &[i32], actions: &[u8]) {
        self.packs = packs.to_vec().into();
        self.actions = actions.to_vec().into();
        let mut p = player.clone();
        let mut turn = 0;
        self.expected_results = self.actions.iter().zip(self.packs.iter()).map(|(a, pack)| {
            let result = p.put(pack, &a.into());
            if turn < enemy_send_obstacles.len() {
                p.add_obstacles(enemy_send_obstacles[turn]);
            }
            turn += 1;
            result
        }).collect();
    }

    pub fn replay(&mut self) -> Option<action::Action> {
        self.packs.pop_front();
        self.expected_results.pop_front();
        self.actions.pop_front().map(|a| a.into())
    }

    pub fn clear(&mut self) {
        self.packs.clear();
        self.actions.clear();
        self.expected_results.clear();
    }

    pub fn get_actions(&self) -> Vec<action::Action> {
        self.actions.iter().map(|a| a.into()).collect()
    }

    pub fn get_results(&self) -> Vec<action::ActionResult> {
        self.expected_results.clone().into()
    }

    pub fn get_obstacles(&self, player: &player::Player) -> Vec<i32> {
        let mut obstacle = player.obstacle;
        self.get_results().into_iter().map(|r| {
            if obstacle >= W as i32 {
                obstacle -= W as i32;
            }
            obstacle -= r.obstacle;
            let result = std::cmp::max(-obstacle, 0);
            obstacle = std::cmp::max(obstacle, 0);
            result
        }).collect()
    }

    pub fn get_obstacles_score(&self, player: &player::Player) -> i32 {
        // let mut fall = 0;
        let mut obstacle = player.obstacle;
        // self.get_results().into_iter().for_each(|r| {
        //     if obstacle >= W as i32 {
        //         obstacle -= W as i32;
        //         fall += W as i32;
        //     }
        //     obstacle -= r.obstacle;
        // });
        // // -obstacle - fall
        // -obstacle
        self.get_results().into_iter().map(|r| r.obstacle).sum::<i32>() - obstacle
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }
}
