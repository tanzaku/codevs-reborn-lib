
use super::action;
use super::board;
use super::player;
// use super::rand;

use std::collections::VecDeque;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::time::{Duration, Instant};
use std::cmp::Reverse;

use super::consts::{W,H};


#[derive(Clone, Default, PartialEq, Eq)]
pub struct BeamState {
    pub player: player::Player,
    pub score: i32,
    pub actions: Vec<u8>,
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
    pub packs: Vec<[[u8; 2]; 2]>,
    pub stop_search_if_3_chains: bool,
}

// pub fn calc_rensa_plan(&mut self, cur_turn: usize, max_fire_turn: usize, player: &player::Player, ) {
pub fn calc_rensa_plan<F>(context: &PlanContext, mut calc_score: F) -> Vec<(BeamState, action::ActionResult)>
    where F: FnMut(&action::ActionResult, &player::Player, usize) -> i32
{
    let timer = Instant::now();

    // let max_fire_turn = if cur_turn == 0 { 13 } else { 10 };
    let actions = action::Action::all_actions();
    // let allow_dead_line = Self::is_dangerous(&player.board);

    let mut heaps = vec![BinaryHeap::new(); context.max_turn];
    // let mut candidates = Vec::new();

    let initial_state = BeamState::new(context.player.clone(), 0, Vec::new());
    heaps[0].push(initial_state.clone());
    let mut bests = vec![(initial_state, action::ActionResult::new(0, 0, 0)); context.max_turn];
    loop {
        let elapsed = timer.elapsed();
        if elapsed.as_secs() >= context.think_time_in_sec {
            break;
        }

        (0..context.max_turn).for_each(|search_turn| {
            let turn = context.plan_start_turn + search_turn;
            let pack = context.packs[turn].clone();
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

                    let stop_search = result.chains >= 3;

                    let mut actions = b.actions.clone();
                    actions.push(a.into());

                    let score = calc_score(&result, &player, search_turn);
                    if bests[search_turn].0.score < score {
                        bests[search_turn] = (BeamState::new(player.clone(), score, actions.clone()), result);
                    }

                    if context.stop_search_if_3_chains && stop_search {
                        return;
                    }

                    // ここら辺の判断は外に出す
                    // if context.plan_start_turn == 0 && player.board.max_height() >= H - 3 {
                    //     return;
                    // }

                    if search_turn + 1 < context.max_turn {
                        let max_score = (0..W).map(|x| (1..=9).map(|v| {
                            let mut rensa_eval_board = player.clone();
                            // let result = rensa_eval_board.put(&fall, &fire_action);
                            let result = rensa_eval_board.put_one(v, x as usize);
                            calc_score(&result, &player, search_turn)
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
    // let elapsed = timer.elapsed();
    // let elapsed = format!("{}.{:03}", elapsed.as_secs(), elapsed.subsec_nanos() / 1_000_000);
    // eprintln!("best: {} {} {}[s]", context.plan_start_turn, best.score, elapsed);
    bests
}

