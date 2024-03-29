

use std::convert::From;


#[derive(Eq, PartialEq, Clone)]
pub enum Action {
    PutBlock { pos: usize, rot: usize },
    UseSkill,
}

#[derive(Eq, PartialEq, Clone, Default)]
pub struct ActionResult {
    pub chains: u8,
    pub obstacle: i32,
    pub skill_guage: i32,
    pub fire_height: i8,
}

impl Action {
    pub fn all_actions() -> Vec<Action> {
        let mut actions = Vec::new();
        for pos in 0..9 {
            for rot in 0..4 {
                actions.push(Action::PutBlock { pos, rot });
            }
        }
        actions.push(Action::UseSkill);
        actions
    }
}

impl From<u8> for Action {
    fn from(item: u8) -> Self {
        let item = item - 1;
        if item == 9 * 4 {
            Action::UseSkill
        } else if item < 9 * 4 {
            let pos = (item / 4) as usize;
            let rot = (item % 4) as usize;
            Action::PutBlock { pos, rot }
        } else {
            unreachable!()
        }
    }
}

impl From<&u8> for Action {
    fn from(item: &u8) -> Self {
        Action::from(*item)
    }
}

impl From<&Action> for u8 {
    fn from(item: &Action) -> Self {
        match item {
            Action::PutBlock { pos, rot } => (pos*4+rot+1) as u8,
            Action::UseSkill => 9*4+1,
        }
    }
}

impl From<&Action> for u128 {
    fn from(item: &Action) -> Self {
        match item {
            Action::PutBlock { pos, rot } => (pos*4+rot+1) as u128,
            Action::UseSkill => 9*4+1,
        }
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Action::PutBlock { pos, rot } => write!(f, "{} {}", pos, rot),
            Action::UseSkill => write!(f, "S"),
        }
    }
}


impl ActionResult {
    pub fn new(chains: u8, obstacle: i32, skill_guage: i32, fire_height: i8) -> Self {
        Self { chains, obstacle, skill_guage, fire_height, }
    }
}
