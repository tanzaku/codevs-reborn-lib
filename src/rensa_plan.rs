
use super::action;
use super::board;
use super::player;
// use super::rand;
use super::rand;

use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::time::{Instant};

use super::consts::*;
use std::collections::HashSet;
use std::collections::HashMap;

use rayon::prelude::*;


#[derive(Clone, Default, PartialEq, Eq)]
pub struct BeamState {
    pub player: player::Player,
    pub score: i64,
    chains: u8,
    pub actions: u128,
}

impl BeamState {
    fn new(player: player::Player, score: i64, chains: u8, actions: u128) -> Self {
        Self { player, score, chains, actions, }
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

const SECOND_CHAINS_SCORE: bool = true;

fn do_action<F>(player: &mut player::Player, search_turn: usize, context: &PlanContext, action: &action::Action, calc_score: &F) -> (action::ActionResult, action::ActionResult, i64)
    where F: Fn(&action::ActionResult, &player::Player, &board::Feature) -> i64 + Sync + Send
{
    let turn = context.plan_start_turn + search_turn;
    let pack = &context.packs[turn];
    let result = player.put(pack, action);

    if search_turn < context.enemy_send_obstacles.len() {
        player.add_obstacles(context.enemy_send_obstacles[search_turn]);
    }

    let feature = player.board.calc_feature();
    let eval_result = player.board.calc_max_rensa_by_erase_outer_block().1;
    let score = calc_score(&eval_result, player, &feature);
    (eval_result, result, score)
}

// pub fn calc_rensa_plan(&mut self, cur_turn: usize, max_fire_turn: usize, player: &player::Player, ) {
pub fn calc_rensa_plan<F>(context: &PlanContext, rand: &mut rand::XorShiftL, calc_score: F) -> Vec<(BeamState, action::ActionResult)>
    where F: Fn(&action::ActionResult, &player::Player, &board::Feature) -> i64 + Sync + Send
{
    assert!(context.max_turn <= 16);
    let timer = Instant::now();

    let actions = action::Action::all_actions();
    let mut heaps = vec![BinaryHeap::new(); context.max_turn];

    let initial_state = BeamState::new(context.player.clone(), 0, 0, 0, 0);
    let mut bests = vec![(initial_state.clone(), action::ActionResult::new(0, 0, 0, 0, 0)); context.max_turn];
    heaps[0].push(initial_state);

    let mut visited = HashSet::new();
    visited.insert(context.player.hash());

    // let mut remove_hashes: Vec<HashMap<u64, u8>> = vec![HashMap::new(); context.max_turn];
    let mut iter = 0;
    loop {
        let elapsed = timer.elapsed();
        let milli = elapsed.as_secs() * 1000 + (elapsed.subsec_nanos() / 1000_000) as u64;
        if milli >= context.think_time_in_milli {
            break;
        }

        iter += 1;
        let mut empty_all = true;
        let mut max_chains = 0;

        (0..context.max_turn).for_each(|search_turn| {
            let turn = context.plan_start_turn + search_turn;
            empty_all &= heaps[search_turn].is_empty();

            // eprintln!("come: {} {}", search_turn, heaps[search_turn].len());
            // if let Some(mut b) = heaps[search_turn].pop() {
            if let Some(b) = heaps[search_turn].pop() {
                // if b.remove_hash != 0 {
                //     let h = remove_hashes[search_turn].get(&b.remove_hash).map(|c| *c).unwrap_or_default();
                //     if h >= 5*1 {
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
                        if let action::Action::PutBlock { pos, rot: _ } = a {
                            if turn == 0 && *pos != W / 2 {
                                return None;
                            }
                        }
                    }

                    let mut player = b.player.clone();
                    let (eval_result, result, score) = do_action(&mut player, search_turn, context, a, &calc_score);
                    let actions = push_action(b.actions, a);
                    
                    Some((player, score, actions, result, eval_result))
                }).collect();

                result.into_iter().filter(|x| x.is_some()).map(|x| x.unwrap()).for_each(|x| {
                    let (player, score, actions, result, eval_result) = x;
                    if player.board.is_dead() || !visited.insert(player.hash()) {
                    // if player.board.is_dead() {
                        return;
                    }
                    // eprintln!("check: {} {}", search_turn, score);
                    // if bests[search_turn].0.score < score {
                    //     bests[search_turn] = (BeamState::new(player.clone(), score + (rand.next() & 0xFF) as i64, 0, result.chains, actions), result.clone());
                    // }
                    let score = score + (rand.next() & 0xFF) as i64;
                    // if bests[search_turn].0.chains < result.chains {
                    if bests[search_turn].0.score < score {
                        bests[search_turn] = (BeamState::new(player.clone(), score, result.chains, actions), result.clone());
                    }
                    // if search_turn + 1 < context.max_turn && result.chains <= 1 {
                    // if search_turn + 1 < context.max_turn && result.chains <= 0 {
                    if search_turn + 1 < context.max_turn {
                        heaps[search_turn + 1].push(BeamState::new(player, score, eval_result.chains, actions));
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
            max_chains = std::cmp::max(max_chains, bests[search_turn].1.chains);
        });

        if empty_all {
            break;
        }
    }

    // bests.iter().for_each(|b| {
    //     eprintln!("beam: {} {}", b.0.score, b.1.chains);
    // });
    if context.verbose {
        eprintln!("iter={}", iter);
    }
    bests
}

