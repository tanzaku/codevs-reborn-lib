
use super::action;
use super::board;
use super::player;
// use super::rand;
use super::rand;
use super::replay;

use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::time::{Instant};

use super::consts::*;
use std::collections::HashSet;
// use hashbrown::HashSet;

// 探索結果
#[derive(Clone, Default, PartialEq, Eq)]
struct SearchResult {
    score: i64,
    actions: u128,
}

impl SearchResult {
    fn get_actions(&self) -> Vec<action::Action> {
        let b = (128 - self.actions.leading_zeros() + 7) / 8;
        let mut a = self.actions;
        let mut res = Vec::with_capacity(b as usize);
        while a != 0 {
            res.push(((a & 0xFF) as u8).into());
            a >>= 8;
        }
        res
    }
}

// ビームサーチ状態
#[derive(Clone, Default, PartialEq, Eq)]
struct BeamState {
    player: player::Player,
    score: i64,
    actions: u128,
}

impl BeamState {
    fn new(player: player::Player, score: i64, actions: u128) -> Self {
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

fn push_action(actions: u128, a: &action::Action) -> u128 {
    let b = (128 - actions.leading_zeros() + 7) / 8;
    let a: u128 = a.into();
    actions | a << (b * 8)
}

pub struct PlanContext<'a> {
    pub plan_start_turn: usize,
    pub max_turn: usize,
    pub think_time_in_milli: u64,
    pub player: player::Player,
    pub enemy_send_obstacles: &'a [i32],
    pub packs: &'a [[[u8; 2]; 2]],
}

// 一手進める
fn do_action<F>(player: &mut player::Player, search_turn: usize, context: &PlanContext, action: &action::Action, calc_score: &F) -> (i64, i64)
    where F: Fn(&action::ActionResult, &player::Player, &board::Feature) -> i64 + Sync + Send
{
    let turn = context.plan_start_turn + search_turn;
    let pack = &context.packs[turn];
    let result = player.put(pack, action);

    if search_turn < context.enemy_send_obstacles.len() {
        player.add_obstacles(context.enemy_send_obstacles[search_turn]);
    }

    let feature = player.board.calc_feature();
    let eval_result = player.board.calc_max_rensa_by_erase_block().1;
    let score = calc_score(&result, player, &feature);
    let eval_score = calc_score(&eval_result, player, &feature);
    (score, eval_score)
}

// ビームサーチ
pub fn calc_rensa_plan<F>(context: &PlanContext, rand: &mut rand::XorShiftL, calc_score: F) -> Vec<replay::Replay>
    where F: Fn(&action::ActionResult, &player::Player, &board::Feature) -> i64 + Sync + Send
{
    assert!(context.max_turn <= 16);
    let timer = Instant::now();

    let actions = action::Action::all_actions();
    let mut heaps = vec![BinaryHeap::new(); context.max_turn];

    let mut bests: Vec<SearchResult> = vec![Default::default(); context.max_turn];
    let initial_state = BeamState::new(context.player.clone(), 0, 0);
    heaps[0].push(initial_state);

    let mut visited = HashSet::new();

    let board_is_empty = context.player.board.is_empty();
    let mut _iter = 0;
    loop {
        let elapsed = timer.elapsed();
        let milli = elapsed.as_secs() * 1000 + (elapsed.subsec_nanos() / 1000_000) as u64;
        if milli >= context.think_time_in_milli || heaps.iter().all(|h| h.is_empty()) {
            break;
        }

        _iter += 1;

        (0..context.max_turn).for_each(|search_turn| {
            let turn = context.plan_start_turn + search_turn;

            if let Some(b) = heaps[search_turn].pop() {
                actions.iter().for_each(|a| {
                    if &action::Action::UseSkill == a && !b.player.can_use_skill() {
                        return;
                    }

                    if board_is_empty && turn == context.plan_start_turn {
                        if let action::Action::PutBlock { pos, rot: _ } = a {
                            if *pos != W / 2 {
                                return;
                            }
                        }
                    }

                    let mut player = b.player.clone();
                    let (score, eval_score) = do_action(&mut player, search_turn, context, a, &calc_score);
                    let actions = push_action(b.actions, a);
                    
                    // if player.board.is_dead() || !context.enemy_send_obstacles.is_empty() && !visited.insert(player.hash()) {
                    if player.board.is_dead() || !visited.insert(player.hash()) {
                    // if player.board.is_dead() {
                        return;
                    }
                    let score = score * 256 + (rand.next() & 0xFF) as i64;
                    let eval_score = eval_score * 256 + (rand.next() & 0xFF) as i64;
                    if search_turn + 1 < context.max_turn {
                        heaps[search_turn + 1].push(BeamState::new(player.clone(), eval_score, actions));
                    }
                    if bests[search_turn].score < score {
                        bests[search_turn] = SearchResult { score, actions, };
                    }
                });
            };
        });
    }

    // eprintln!("iter={}", _iter);
    // bests.iter().for_each(|b| { eprintln!("obstacle={}", b.0.score / 10000000000); });
    bests.into_iter().map(|b| {
        let mut replay = replay::Replay::new();
        let actions = b.get_actions();
        let start_turn = context.plan_start_turn;
        let last_turn = start_turn + actions.len();
        replay.init(&context.player, &context.packs[start_turn..last_turn], context.enemy_send_obstacles, &actions);
        replay
    }).collect()
}

