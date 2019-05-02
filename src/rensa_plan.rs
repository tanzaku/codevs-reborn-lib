
use super::action;
use super::board;
use super::player;
// use super::rand;
use super::rand;

use std::collections::VecDeque;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::time::{Duration, Instant};
use std::cmp::Reverse;

use super::consts::{W,H};
use std::collections::HashSet;
use std::collections::HashMap;

use rayon::prelude::*;


#[derive(Clone, Default, PartialEq, Eq)]
pub struct BeamState {
    pub player: player::Player,
    pub score: i64,
    remove_hash: u64,
    pub actions: u128,
}

impl BeamState {
    fn new(player: player::Player, score: i64, remove_hash: u64, actions: u128) -> Self {
        Self { player, score, remove_hash, actions, }
    }
    pub fn get_actions(&self) -> Vec<u8> {
        let b = (128 - self.actions.leading_zeros() + 7) / 8;
        let mut a = self.actions;
        let mut res = vec![0; b as usize];
        let mut i = 0;
        while a != 0 {
            res[i] = (a & 0xFF) as u8;
            i += 1;
            a >>= 8;
        }
        res
    }
}

fn push_action(actions: u128, a: &action::Action) -> u128 {
    let b = (128 - actions.leading_zeros() + 7) / 8;
    let a: u128 = a.into();
    // eprintln!("debug: {} {} {}", actions, b, a);
    actions | a << (b * 8)
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
    pub think_time_in_milli: u64,
    pub player: player::Player,
    pub enemy_send_obstacles: Vec<i32>,
    pub packs: Vec<[[u8; 2]; 2]>,
    pub stop_search_if_3_chains: bool,
    pub replay: Vec<action::Action>,
    pub verbose: bool,
}

fn fire<F>(player: &player::Player, feature: &board::Feature, calc_score: &F) -> (i64, action::ActionResult, usize, u64)
    where F: Fn(&action::ActionResult, i32, &player::Player, &board::Feature) -> i64 + Sync + Send
{
    (0..W).map(|x| (1..=9).map(|v| {
        let mut rensa_eval_board = player.clone();
        let result = rensa_eval_board.put_one(v, x);
        let t: (i64, action::ActionResult, usize, u64) = (calc_score(&result, 0, player, feature), result, x, v);
        t
    }).max_by_key(|x| x.0).unwrap()).max_by_key(|x| x.0).unwrap()
}

fn eval<F>(player: &player::Player, feature: &board::Feature, calc_score: &F) -> (i64, u64)
    where F: Fn(&action::ActionResult, i32, &player::Player, &board::Feature) -> i64 + Sync + Send
{
    let best = fire(player, feature, calc_score);
    let mut player_put = player.clone();
    player_put.put_one(best.3, best.2 as usize);

    // let max_score = (0..W).flat_map(|x| (1..=9).map(|v| {
    // })).max_by_key(|x| x.0).unwrap();

    // (0..W).flat_map(|x| (1..=9).map(|v| {
    //     let mut rensa_eval_board = player_put.clone();
    //     let result = rensa_eval_board.put_one(v, x as usize);
    //     (calc_score(&best.1, result.chains, player, feature), best.1.remove_hash)
    // })).max_by_key(|x| x.0).unwrap()

   (0..W).map(|x| (1..=9).map(|v| {
        let mut rensa_eval_board = player_put.clone();
        let result = rensa_eval_board.put_one(v, x);
        let t: (i64, u64) = (calc_score(&best.1, result.chains as i32, player, feature), best.1.remove_hash);
        t
    }).max_by_key(|x| x.0).unwrap()).max_by_key(|x| x.0).unwrap()
}

fn do_action<F>(player: &mut player::Player, pack: &[[u8; 2]; 2], action: &action::Action, calc_score: &F) -> ((i64, u64), action::ActionResult, board::Feature, i64)
    where F: Fn(&action::ActionResult, i32, &player::Player, &board::Feature) -> i64 + Sync + Send
{
    let result = player.put(pack, action);

    // let max_score = (0..W).flat_map(|x| (1..=9).map(|v| {
    // })).max_by_key(|x| x.0).unwrap();
    let feature = player.board.calc_feature();
    let score = (0..W).map(|x| (1..=9).map(|v| {
        let mut rensa_eval_board = player.clone();
        let second_result = rensa_eval_board.put_one(v, x);
        calc_score(&result, second_result.chains as i32, player, &feature)
    }).max().unwrap()).max().unwrap();

    let eval_result = eval(player, &feature, &calc_score);
    (eval_result, result, feature, score)
}

// pub fn calc_rensa_plan(&mut self, cur_turn: usize, max_fire_turn: usize, player: &player::Player, ) {
pub fn calc_rensa_plan<F>(context: &PlanContext, rand: &mut rand::XorShiftL, calc_score: F) -> Vec<(BeamState, action::ActionResult)>
    where F: Fn(&action::ActionResult, i32, &player::Player, &board::Feature) -> i64 + Sync + Send
{
    assert!(context.max_turn <= 16);
    let timer = Instant::now();

    // let max_fire_turn = if cur_turn == 0 { 13 } else { 10 };
    let actions = action::Action::all_actions();
    // let allow_dead_line = Self::is_dangerous(&player.board);

    let mut heaps = vec![BinaryHeap::new(); context.max_turn];
    // let mut candidates = Vec::new();

    let initial_state = BeamState::new(context.player.clone(), 0, 0, 0);
    let mut bests = vec![(initial_state.clone(), action::ActionResult::new(0, 0, 0, 0, 0)); context.max_turn];
    heaps[0].push(initial_state);

    let mut visited = HashSet::new();
    visited.insert(context.player.hash());

    // for search_turn in 0..context.replay.len() {
    //     if let Some(b) = heaps[search_turn].peek() {
    //         let turn = context.plan_start_turn + search_turn;

    //         let mut player = b.player.clone();
    //         if search_turn > 0 && search_turn - 1 < context.enemy_send_obstacles.len() {
    //             player.add_obstacles(context.enemy_send_obstacles[search_turn - 1]);
    //         }
            
    //         let pack = context.packs[turn].clone();
    //         let a = &context.replay[search_turn];
    //         let result = player.put(&pack, a);
    //         if player.board.is_dead() {
    //             break;
    //         }

    //         visited.insert(player.hash());
    //         let actions = push_action(b.actions, a);

    //         let feature = player.board.calc_feature();
    //         let score = calc_score(&result, &player, search_turn, &feature);
    //         if bests[search_turn].0.score < score {
    //             bests[search_turn] = (BeamState::new(player.clone(), score, result.remove_hash, actions), result);
    //         }

    //         if search_turn + 1 < context.max_turn {
    //             let max_score = eval(&player, &feature, calc_score);
    //             heaps[search_turn + 1].push(BeamState::new(player.clone(), max_score.0 + (rand.next() & 0xFF) as i32, max_score.1.remove_hash, actions));
    //         }
    //     }
    // }

    let mut remove_hashes: Vec<HashMap<u64, u8>> = vec![HashMap::new(); context.max_turn];
    let mut iter = 0;
    loop {
        let elapsed = timer.elapsed();
        let milli = elapsed.as_secs() * 1000 + (elapsed.subsec_nanos() / 1000_000) as u64;
        if milli >= context.think_time_in_milli {
            break;
        }

        iter += 1;
        let mut empty_all = true;

        (0..context.max_turn).for_each(|search_turn| {
            let turn = context.plan_start_turn + search_turn;
            let pack = context.packs[turn].clone();

            empty_all &= heaps[search_turn].is_empty();

            // eprintln!("come: {} {}", search_turn, heaps[search_turn].len());
            if let Some(mut b) = heaps[search_turn].pop() {
                if search_turn > 0 && search_turn - 1 < context.enemy_send_obstacles.len() {
                    b.player.add_obstacles(context.enemy_send_obstacles[search_turn - 1]);
                }

                // if b.remove_hash != 0 {
                //     let h = remove_hashes[search_turn].get(&b.remove_hash).map(|c| *c).unwrap_or_default();
                //     if h >= 5 {
                //         // eprintln!("branch cut: {}", b.remove_hash);
                //         return;
                //     }
                //     remove_hashes[search_turn].insert(b.remove_hash, h + 1);
                // }

                let result: Vec<_> = actions.par_iter().map(|a| {
                    if &action::Action::UseSkill == a && !b.player.can_use_skill() {
                        return None;
                    }

                    if context.plan_start_turn == 0 {
                        if let action::Action::PutBlock { pos, rot } = a {
                            if turn == 0 && *pos != W / 2 {
                                return None;
                            }
                        }
                    }

                    let mut player = b.player.clone();
                    let (max_score, result, feature, score) = do_action(&mut player, &pack, a, &calc_score);
                    let actions = push_action(b.actions, a);
                    
                    Some((player, score, max_score, actions, result))
                }).collect();

                result.into_iter().filter(|x| x.is_some()).map(|x| x.unwrap()).for_each(|x| {
                    let (player, score, max_score, actions, result) = x;
                    if player.board.is_dead() || !visited.insert(player.hash()) {
                        return;
                    }
                    // eprintln!("check: {} {}", search_turn, score);
                    if bests[search_turn].0.score < score {
                        bests[search_turn] = (BeamState::new(player.clone(), score + (rand.next() & 0xFF) as i64, 0, actions), result);
                    }
                    if search_turn + 1 < context.max_turn {
                        heaps[search_turn + 1].push(BeamState::new(player, max_score.0 + (rand.next() & 0xFF) as i64, max_score.1, actions));
                    }
                });
                    // if player.board.is_dead() || !visited.insert(player.hash()) {
                    //     return (None, None);
                    // }
                    // if bests[search_turn].0.score < score {
                    //     bests[search_turn] = (BeamState::new(player.clone(), score, 0, actions), result);
                    // }
                    // Some(BeamState::new(player, max_score.0, max_score.1.remove_hash, actions))
            };
        });

        if empty_all {
            break;
        }
    }

    // bests.iter().for_each(|b| {
    //     eprintln!("beam: {} {}", b.0.score, b.1.chains);
    // });
    if context.verbose {
        // eprintln!("iter={}", iter);
    }
    bests
}

