
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
use std::collections::HashSet;
use std::collections::HashMap;


#[derive(Clone, Default, PartialEq, Eq)]
pub struct BeamState {
    pub player: player::Player,
    pub score: i32,
    // pub remove_hash: u64,
    pub actions: Vec<u8>,
}

impl BeamState {
    fn new(player: player::Player, score: i32, remove_hash: u64, actions: Vec<u8>) -> Self {
        Self { player, score, /*remove_hash,*/ actions, }
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
    pub replay: Vec<action::Action>,
    pub verbose: bool,
}

// pub fn calc_rensa_plan(&mut self, cur_turn: usize, max_fire_turn: usize, player: &player::Player, ) {
pub fn calc_rensa_plan<F>(context: &PlanContext, mut calc_score: F) -> Vec<(BeamState, action::ActionResult)>
    where F: FnMut(&action::ActionResult, &player::Player, usize, &board::Feature) -> i32
{
    let timer = Instant::now();

    // let max_fire_turn = if cur_turn == 0 { 13 } else { 10 };
    let actions = action::Action::all_actions();
    // let allow_dead_line = Self::is_dangerous(&player.board);

    let mut heaps = vec![BinaryHeap::new(); context.max_turn];
    // let mut candidates = Vec::new();

    let initial_state: BeamState = Default::default();
    let mut bests = vec![(initial_state.clone(), action::ActionResult::new(0, 0, 0, 0, 0)); context.max_turn];
    heaps[0].push(initial_state);

    let mut visited = HashSet::new();
    visited.insert(context.player.hash());

    // let mut remove_hashes = vec![HashMap::new(); context.max_turn];

    for search_turn in 0..context.replay.len() {
        if let Some(b) = heaps[search_turn].peek() {
            let turn = context.plan_start_turn + search_turn;

            let mut player = b.player.clone();
            if search_turn < context.enemy_send_obstacles.len() {
                player.add_obstacles(context.enemy_send_obstacles[search_turn]);
            }
            
            let pack = context.packs[turn].clone();
            let a = &context.replay[search_turn];
            let result = player.put(&pack, a);
            if player.board.is_dead() {
                break;
            }

            visited.insert(player.hash());
            let mut actions = b.actions.clone();
            actions.push(a.into());

            let feature = player.board.calc_feature();
            let score = calc_score(&result, &player, search_turn, &feature);
            if bests[search_turn].0.score < score {
                bests[search_turn] = (BeamState::new(player.clone(), score, result.remove_hash, actions.clone()), result);
            }

            if search_turn + 1 < context.max_turn {
                let max_score = (0..W).map(|x| (1..=9).map(|v| {
                    let mut rensa_eval_board = player.clone();
                    // let result = rensa_eval_board.put(&fall, &fire_action);
                    let result = rensa_eval_board.put_one(v, x as usize);
                    (calc_score(&result, &player, search_turn, &feature), result)
                }).max_by_key(|x| x.0).unwrap()).max_by_key(|x| x.0).unwrap();

                // let max_score = (0..W).flat_map(|x| (1..=9).map(|v| {
                //     let mut rensa_eval_board = player.clone();
                //     // let result = rensa_eval_board.put(&fall, &fire_action);
                //     let result = rensa_eval_board.put_one(v, x as usize);
                //     (calc_score(&result, &player, search_turn, &feature), result)
                //     // result.obstacle
                // })).max_by_key(|x| x.0).unwrap();
                
                // candidates.push(BeamState::new(player.clone(), max_score, actions.clone()));
                heaps[search_turn + 1].push(BeamState::new(player.clone(), max_score.0, max_score.1.remove_hash, actions.clone()));
            }
        }
    }

    let mut iter = 0;
    loop {
        let elapsed = timer.elapsed();
        if elapsed.as_secs() >= context.think_time_in_sec {
            break;
        }

        let mut empty_all = true;
        iter += 1;

        (0..context.max_turn).for_each(|search_turn| {
            let turn = context.plan_start_turn + search_turn;
            let pack = context.packs[turn].clone();

            empty_all &= heaps[search_turn].is_empty();

            (0..1).for_each(|_| {
                // eprintln!("come: {} {}", search_turn, heaps[search_turn].len());
                if let Some(mut b) = heaps[search_turn].pop() {
                    if search_turn < context.enemy_send_obstacles.len() {
                        b.player.add_obstacles(context.enemy_send_obstacles[search_turn]);
                    }

                    // if b.remove_hash != 0 {
                    //     // let h = remove_hashes[search_turn].get(&b.remove_hash).map(|h: &u64| *h).unwrap_or_default();
                    //     // if h >= 100 {
                    //     //     // eprintln!("branch cut: {}", b.remove_hash);
                    //     //     return;
                    //     // }
                    //     // remove_hashes[search_turn].insert(b.remove_hash, h + 1);
                    // }

                    actions.iter().for_each(|a| {
                        if &action::Action::UseSkill == a && !b.player.can_use_skill() {
                            return;
                        }

                        if context.plan_start_turn == 0 {
                            if let action::Action::PutBlock { pos, rot } = a {
                                if turn == 0 && *pos != W / 2 {
                                    return;
                                }
                                // let last_action = b.actions.last().map(|a| a.into());
                                // if let Some(action::Action::PutBlock { pos: prev_pos, rot: prev_rot }) = last_action {
                                //     let p1 = *pos as i32;
                                //     let p2 = prev_pos as i32;
                                //     // if p2 < p1 - 2 || p2 > p1 + 2 {
                                //     // if p2 < p1 - 3 || p2 > p1 + 3 {
                                //     if p2 < p1 - 5 || p2 > p1 + 5 {
                                //         return;
                                //     }
                                // }
                            }
                        }

                        let mut player = b.player.clone();
                        let result = player.put(&pack, a);

                        // if context.verbose && best < result.chains {
                        //     best = result.chains;
                        //     let elapsed = timer.elapsed();
                        //     eprintln!("improve: {} {}.{:03}", best, elapsed.as_secs(), elapsed.subsec_nanos() / 1_000_000);
                        // }

                        let stop_search = result.chains >= 3;

                        let mut actions = b.actions.clone();
                        actions.push(a.into());

                        let feature = player.board.calc_feature();
                        let score = calc_score(&result, &player, search_turn, &feature);
                        if bests[search_turn].0.score < score {
                            bests[search_turn] = (BeamState::new(player.clone(), score, result.remove_hash, actions.clone()), result);
                        }

                        if player.board.is_dead() || !visited.insert(player.hash()) {
                            return;
                        }

                        // if context.stop_search_if_3_chains && stop_search {
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
                                // (calc_score(&result, &player, search_turn, &feature), result)
                                calc_score(&result, &player, search_turn, &feature)
                            }).max().unwrap()).max().unwrap();
                            
                            // let max_score = (0..W).flat_map(|x| (1..=9).map(|v| {
                            //     let mut rensa_eval_board = player.clone();
                            //     // let result = rensa_eval_board.put(&fall, &fire_action);
                            //     let result = rensa_eval_board.put_one(v, x as usize);
                            //     (calc_score(&result, &player, search_turn, &feature), result)
                            //     // result.obstacle
                            // })).max_by_key(|x| x.0).unwrap();
                            // candidates.push(BeamState::new(player.clone(), max_score, actions.clone()));

                            // heaps[search_turn + 1].push(BeamState::new(player.clone(), max_score.0, max_score.1.remove_hash, actions.clone()));
                            heaps[search_turn + 1].push(BeamState::new(player.clone(), max_score, 0, actions.clone()));
                        }
                    });

                    // candidates.sort();
                    // candidates.iter().rev().take(5).for_each(|b| {
                    //    heaps[search_turn + 1].push(b.clone());
                    // });
                    // candidates.clear();
                };
            });
        });

        if empty_all {
            break;
        }
    }

    eprintln!("iter={}", iter);

    // let elapsed = timer.elapsed();
    // let elapsed = format!("{}.{:03}", elapsed.as_secs(), elapsed.subsec_nanos() / 1_000_000);
    // eprintln!("best: {} {} {}[s]", context.plan_start_turn, best.score, elapsed);
    bests
}
