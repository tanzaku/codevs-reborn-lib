
use super::action;
use super::board;
use super::player;
use super::rand;

use std::collections::VecDeque;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::time::{Duration, Instant};

use super::consts::{W,H};

#[derive(Clone, Default, PartialEq, Eq)]
struct BeamState {
    player: player::Player,
    score: i32,
    actions: Vec<u8>,
}

impl BeamState {
    fn new(player: player::Player, score: i32, actions: Vec<u8>) -> Self {
        Self { player, score, actions, }
    }
}

impl Ord for BeamState {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl PartialOrd for BeamState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct PlanContext {
    pub plan_start_turn: usize,
    pub max_turn: usize,
    pub think_time_in_sec: u64,
    pub player: player::Player,
    pub enemy_send_obstacles: Vec<i32>,
}

pub struct SkillPlan {
    rand: rand::XorShiftL,
    packs: Vec<[[u8; 2]; 2]>,
    replay: VecDeque<action::Action>
}

impl SkillPlan {
    pub fn new() -> Self {
        Self {
            rand: rand::XorShiftL::new(),
            packs: Vec::new(),
            replay: VecDeque::new(),
        }
    }

    pub fn set_pack(&mut self, packs: Vec<[[u8; 2]; 2]>) {
        self.packs = packs;
    }
    
    fn calc_score(&mut self, result: &action::ActionResult, player: &player::Player, search_turn: usize) -> i32 {
        if result.obstacle < 40 {
            (player.skill_guage * 100 + (self.rand.next() & 0xF) as i32)
        } else {
            // (result.obstacle * 10000 + player.skill_guage * 100 + (self.rand.next() & 0xF) as i32)
            // (result.obstacle * 10000 - (search_turn as i32) * 100 + (self.rand.next() & 0xF) as i32)
            (1 * 100000 - (search_turn as i32) * 100 + (self.rand.next() & 0xF) as i32)
        }
    }
    
    pub fn calc_skill_plan(&mut self, context: &PlanContext) -> i32 {
        let timer = Instant::now();

        // let max_fire_turn = if cur_turn == 0 { 13 } else { 10 };
        let actions = action::Action::all_actions();
        // let allow_dead_line = Self::is_dangerous(&player.board);

        let mut heaps = vec![BinaryHeap::new(); context.max_turn];

        let initial_state = BeamState::new(context.player.clone(), 0, Vec::new());
        heaps[0].push(initial_state.clone());
        let mut best = initial_state;
        let mut best_obstacle = 0;
        loop {
            let elapsed = timer.elapsed();
            if elapsed.as_secs() >= context.think_time_in_sec {
                break;
            }

            (0..context.max_turn).for_each(|search_turn| {
                let turn = context.plan_start_turn + search_turn;
                let pack = self.packs[turn].clone();
                // eprintln!("come: {} {}", search_turn, heaps[search_turn].len());
                if let Some(mut b) = heaps[search_turn].pop() {
                    b.player.add_obstacles(context.enemy_send_obstacles[search_turn]);

                    actions.iter().for_each(|a| {
                        if &action::Action::UseSkill == a && !b.player.can_use_skill() {
                            return;
                        }

                        let mut player = b.player.clone();
                        let result = player.put(&pack, a);
                        if player.board.is_dead() {
                            return;
                        }

                        let mut actions = b.actions.clone();
                        actions.push(a.into());

                        let score = self.calc_score(&result, &player, search_turn);
                        if best.score < score {
                            best = BeamState::new(player.clone(), score, actions.clone());
                            best_obstacle = result.obstacle;
                        }

                        if result.chains >= 3 {
                            return;
                        }

                        if context.plan_start_turn == 0 && player.board.max_height() >= H - 3 {
                            return;
                        }

                        if search_turn + 1 < context.max_turn {
                            let rensa_max_score = (0..W).map(|x| (1..=9).map(|v| {
                                let mut skill_eval_board = player.clone();
                                let result = skill_eval_board.put_one(v, x as usize);
                                result.obstacle
                            }).max().unwrap()).max().unwrap();
                            let skill_score = {
                                let mut skill_eval_board = player.clone();
                                let result = skill_eval_board.put(&pack, &action::Action::UseSkill);
                                result.obstacle
                            };
                            let skill_guage = std::cmp::min(player.skill_guage, 80);
                            let max_height = player.board.max_height() as i32;
                            let score = skill_guage * 10000
                                        + skill_score * 1000
                                        - max_height * 100
                                        - rensa_max_score * 500
                                        ;
                            heaps[search_turn + 1].push(BeamState::new(player.clone(), score, actions.clone()));
                        }
                    });
                };
            });
        }
        let elapsed = timer.elapsed();
        let elapsed = format!("{}.{:03}", elapsed.as_secs(), elapsed.subsec_nanos() / 1_000_000);

        eprintln!("best: {} {} {} {}", context.plan_start_turn, best.score, best_obstacle, elapsed);
        self.replay = best.actions.iter().map(|s| action::Action::from(*s)).collect();
        best.score / 10000
    }

    pub fn replay(&mut self) -> action::Action {
        if let Some(a) = self.replay.pop_front() {
            a
        } else {
            action::Action::PutBlock { pos: 0, rot: 0 }
        }
    }

    pub fn exists(&self) -> bool {
        !self.replay.is_empty()
    }

    pub fn can_replay(&self, player: &player::Player) -> bool {
        if let Some(action::Action::UseSkill) = self.replay.front() {
            player.can_use_skill()
        } else {
            self.exists()
        }
    }
}


