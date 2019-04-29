

use std::collections::VecDeque;

use super::action;
use super::board;
use super::player;
use super::rensa_plan;
use super::skill_plan;
use super::consts::{W,H,MAX_TURN};



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

    pub fn can_replay(&self, player: &player::Player) -> bool {
        if self.actions.is_empty() {
            return false;
        }

        let mut p = player.clone();
        let mut illegal_action = false;
        let result = self.actions.iter().zip(self.packs.iter()).map(|(a, pack)| {
            let a = a.into();
            if a == action::Action::UseSkill && !p.can_use_skill() {
                illegal_action = true;
            }
            p.put(pack, &a)
        }).last().unwrap();

        !illegal_action && &result == self.expected_results.back().unwrap()
    }

    pub fn init(&mut self, player: &player::Player, packs: &[[[u8; 2]; 2]], actions: &[u8]) {
        self.packs = packs.to_vec().into();
        self.actions = actions.to_vec().into();
        let mut p = player.clone();
        self.expected_results = self.actions.iter().zip(self.packs.iter()).map(|(a, pack)|  p.put(pack, &a.into())).collect();
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

    pub fn get_obstacles(&self) -> Vec<i32> {
        self.get_results().into_iter().map(|r| r.obstacle).collect()
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }
}
