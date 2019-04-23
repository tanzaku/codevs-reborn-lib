

use std::collections::VecDeque;

use super::action;
use super::board;
use super::player;
use super::rensa_plan;
use super::skill_plan;
use super::consts::{W,H,MAX_TURN};



pub struct Replay {
    expected_result: action::ActionResult,
    packs: VecDeque<[[u8; 2]; 2]>,
    actions: VecDeque<u8>,
}

impl Replay {
    pub fn new() -> Self {
        Self {
            expected_result: action::ActionResult::new(0, 0, 0),
            packs: VecDeque::new(),
            actions: VecDeque::new(),
        }
    }

    pub fn can_replay(&self, player: &player::Player) -> bool {
        if self.actions.is_empty() {
            return false;
        }

        let mut p = player.clone();
        let result = self.actions.iter().zip(self.packs.iter()).map(|(a, pack)| {
            p.put(pack, &a.into())
        }).last().unwrap();

        result == self.expected_result
    }

    pub fn init(&mut self, packs: &[[[u8; 2]; 2]], actions: &[u8], expected_result: &action::ActionResult) {
        self.packs = packs.to_vec().into();
        self.actions = actions.to_vec().into();
        self.expected_result = expected_result.clone();
    }

    pub fn replay(&mut self) -> Option<action::Action> {
        self.packs.pop_front();
        self.actions.pop_front().map(|a| a.into())
    }

    pub fn clear(&mut self) {
        self.packs.clear();
        self.actions.clear();
    }
}
