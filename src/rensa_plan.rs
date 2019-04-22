
use super::action;
use super::board;
use super::player;
use super::rand;

use std::collections::VecDeque;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::time::{Duration, Instant};
use std::cmp::Reverse;

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

pub struct RensaPlan {
    rand: rand::XorShiftL,
    packs: Vec<[[u8; 2]; 2]>,
    replay: VecDeque<action::Action>
}

impl RensaPlan {
    pub fn new() -> Self {
        unsafe {
            use std::arch::x86_64::*;
            let t = _rdtsc();
            Self {
                rand: rand::XorShiftL::from_seed(t as u64),
                packs: Vec::new(),
                replay: VecDeque::new(),
            }
        }
    }

    pub fn set_pack(&mut self, packs: Vec<[[u8; 2]; 2]>) {
        self.packs = packs;
    }
    
    fn calc_score(&mut self, result: &action::ActionResult, b: &board::Board, search_turn: usize) -> i32 {
        // let h = b.max_height() as i32;
        // -(result.obstacle * 10000 - h * 10 + search_turn as i32 * 16 + (self.rand.next() & 0xF) as i32)
        let obstacle_score = std::cmp::min(result.obstacle, 60);
        (obstacle_score * 100000 - search_turn as i32 * 1000 + result.obstacle * 16 + (self.rand.next() & 0xF) as i32)
    }
    
    // pub fn calc_rensa_plan(&mut self, cur_turn: usize, max_fire_turn: usize, player: &player::Player, ) {
    pub fn calc_rensa_plan(&mut self, context: &PlanContext) -> i32 {
        let timer = Instant::now();

        // let max_fire_turn = if cur_turn == 0 { 13 } else { 10 };
        let actions = action::Action::all_actions();
        // let allow_dead_line = Self::is_dangerous(&player.board);

        let mut heaps = vec![BinaryHeap::new(); context.max_turn];
        // let mut candidates = Vec::new();

        let initial_state = BeamState::new(context.player.clone(), 0, Vec::new());
        heaps[0].push(initial_state.clone());
        let mut best = initial_state;
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

                        if turn == 0 {
                            if let action::Action::PutBlock { pos, rot } = a {
                                if *pos != W / 2 {
                                    return;
                                }
                            }
                        }

                        if turn == 1 {
                            if let action::Action::PutBlock { pos, rot } = a {
                                if *pos < W / 2 - 1 || *pos > W / 2 + 1 {
                                    return;
                                }
                            }
                        }

                        let mut player = b.player.clone();
                        let result = player.put(&pack, a);
                        if player.board.is_dead() {
                            return;
                        }

                        let mut actions = b.actions.clone();
                        actions.push(a.into());

                        let score = self.calc_score(&result, &player.board, search_turn);
                        if best.score < score {
                            best = BeamState::new(player.clone(), score, actions.clone());
                        }

                        // if result.chains >= 3 {
                        //     return;
                        // }

                        // ここら辺の判断は外に出す
                        // if context.plan_start_turn == 0 && player.board.max_height() >= H - 3 {
                        //     return;
                        // }

                        if search_turn + 1 < context.max_turn {
                            let max_score = (0..W).map(|x| (1..=9).map(|v| {
                                let mut rensa_eval_board = player.clone();
                                // let result = rensa_eval_board.put(&fall, &fire_action);
                                let result = rensa_eval_board.put_one(v, x as usize);
                                self.calc_score(&result, &player.board, search_turn)
                            }).max().unwrap()).max().unwrap();
                            // candidates.push(BeamState::new(player.clone(), max_score, actions.clone()));
                            heaps[search_turn + 1].push(BeamState::new(player.clone(), max_score, actions.clone()));
                        }
                    });

                    // candidates.sort();
                    // candidates.iter().rev().take(20).for_each(|b| {
                    //    heaps[search_turn + 1].push(b.clone());
                    // });
                    // candidates.clear();
                };
            });
        }
        let elapsed = timer.elapsed();
        let elapsed = format!("{}.{:03}", elapsed.as_secs(), elapsed.subsec_nanos() / 1_000_000);

        eprintln!("best: {} {} {}[s]", context.plan_start_turn, best.score, elapsed);
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

    pub fn clear_replay(&mut self) {
        self.replay.clear();
    }
}


