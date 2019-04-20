
use super::action;
use super::board;
use super::player;
use super::rand;

use std::collections::VecDeque;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::time::{Duration, Instant};

const DEAD_LINE_Y: i32 = 16;

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

pub struct RensaPlan {
    rand: rand::XorShiftL,
    packs: Vec<[[u8; 2]; 2]>,
    replay: VecDeque<action::Action>
}

impl RensaPlan {
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

    // fn is_dangerous(board: &board::Board) -> bool {
    //     board.max_height() as i32 >= DEAD_LINE_Y - 1
    // }

    fn calc_score(&mut self, result: &action::ActionResult, b: &board::Board, search_turn: usize) -> i32 {
        // let h = b.max_height() as i32;
        // -(result.obstacle * 10000 - h * 10 + search_turn as i32 * 16 + (self.rand.next() & 0xF) as i32)
        -(result.obstacle * 10000 + search_turn as i32 * 16 + (self.rand.next() & 0xF) as i32)
    }
    
    fn calc_score_chokudai(&mut self, result: &action::ActionResult, b: &board::Board, search_turn: usize) -> i32 {
        // let h = b.max_height() as i32;
        // -(result.obstacle * 10000 - h * 10 + search_turn as i32 * 16 + (self.rand.next() & 0xF) as i32)
        (result.obstacle * 10000 + search_turn as i32 * 16 + (self.rand.next() & 0xF) as i32)
    }
    
    pub fn calc_rensa_plan(&mut self, cur_turn: usize, player: &player::Player) {
        let timer = Instant::now();

        let max_fire_turn = if cur_turn == 0 { 13 } else { 10 };
        let actions = action::Action::all_actions();
        // let allow_dead_line = Self::is_dangerous(&player.board);

        let mut heaps = vec![BinaryHeap::new(); max_fire_turn];

        let initial_state = BeamState::new(player.clone(), 0, Vec::new());
        heaps[0].push(initial_state.clone());
        let mut best = initial_state;
        loop {
            let elapsed = timer.elapsed();
            if elapsed.as_secs() >= 15 {
                break;
            }

            (0..max_fire_turn).for_each(|search_turn| {
                let turn = cur_turn + search_turn;
                let pack = self.packs[turn].clone();
                // eprintln!("come: {} {}", search_turn, heaps[search_turn].len());
                if let Some(b) = heaps[search_turn].pop() {
                    actions.iter().for_each(|a| {
                        if let action::Action::UseSkill = a {
                            return;
                            // if !b.player.can_use_skill() {
                            //     return;
                            // }
                        }

                        let mut player = b.player.clone();
                        let result = player.put(&pack, a);
                        if player.board.is_dead() {
                            return;
                        }

                        let mut actions = b.actions.clone();
                        actions.push(a.into());

                        let score = self.calc_score_chokudai(&result, &player.board, search_turn);
                        if best.score < score {
                            best = BeamState::new(player.clone(), score, actions.clone());
                        }

                        if result.chains >= 3 {
                            return;
                        }

                        if cur_turn == 0 && player.board.max_height() >= H - 3 {
                            return;
                        }

                        // obstacleが降ってくると危ないので
                        // if !allow_dead_line && Self::is_dangerous(&board) {
                        //     return;
                        // }

                        if search_turn + 1 < max_fire_turn {
                            let max_score = (0..W).map(|x| (1..=9).map(|v| {
                                let mut rensa_eval_board = player.clone();
                                // let result = rensa_eval_board.put(&fall, &fire_action);
                                let result = rensa_eval_board.put_one(v, x as usize);
                                self.calc_score_chokudai(&result, &player.board, search_turn)
                            }).max().unwrap()).max().unwrap();
                            heaps[search_turn + 1].push(BeamState::new(player.clone(), max_score, actions.clone()));
                        }
                    });
                };
            });
        }
        let elapsed = timer.elapsed();
        let elapsed = format!("{}.{:03}", elapsed.as_secs(), elapsed.subsec_nanos() / 1_000_000);

        eprintln!("best: {} {} {}[s]", cur_turn, best.score, elapsed);
        let best = best.actions;

        self.replay = best.iter().map(|s| action::Action::from(*s)).collect();
    }

    pub fn calc_rensa_plan_beam(&mut self, cur_turn: usize, player: &player::Player) {
        let timer = Instant::now();

        let mut next = Vec::new();
        let mut cur = vec![BeamState::new(player.clone(), 0, Vec::new())];
        let max_fire_turn = if cur_turn == 0 { 13 } else { 10 };
        let beam_width = 100 * 3 * 3 * 3 * 1;
        let beam_width = if cur_turn == 0 { 100 * 3 * 3 * 2 } else { 100 * 3 * 3 * 3 };
        let actions = action::Action::all_actions();
        // let allow_dead_line = Self::is_dangerous(&player.board);

        let mut best = BeamState::new(player.clone(), 0, Vec::new());
        for search_turn in 0..max_fire_turn {
            let turn = cur_turn + search_turn;
            let pack = self.packs[turn].clone();
            cur.iter().for_each(|b| {
                actions.iter().for_each(|a| {
                    if let action::Action::UseSkill = a {
                        return;
                        // if !b.player.can_use_skill() {
                        //     return;
                        // }
                    }

                    let mut player = b.player.clone();
                    let result = player.put(&pack, a);
                    if player.board.is_dead() {
                        return;
                    }

                    let mut actions = b.actions.clone();
                    actions.push(a.into());

                    let score = self.calc_score(&result, &player.board, search_turn);
                    if best.score > score {
                        best = BeamState::new(player.clone(), score, actions.clone());
                    }

                    if result.chains >= 3 {
                        return;
                    }

                    if cur_turn == 0 && player.board.max_height() >= H - 3 {
                        return;
                    }

                    // obstacleが降ってくると危ないので
                    // if !allow_dead_line && Self::is_dangerous(&board) {
                    //     return;
                    // }

                    let min_score = (0..W).map(|x| (1..=9).map(|v| {
                        let mut rensa_eval_board = player.clone();
                        // let result = rensa_eval_board.put(&fall, &fire_action);
                        let result = rensa_eval_board.put_one(v, x as usize);
                        self.calc_score(&result, &player.board, search_turn)
                    }).min().unwrap()).min().unwrap();

                    next.push(BeamState::new(player.clone(), min_score, actions.clone()));
                });
            });
            next.sort();
            next.resize(beam_width, Default::default());
            std::mem::swap(&mut cur, &mut next);
            next.clear();
        }
        let elapsed = timer.elapsed();
        let elapsed = format!("{}.{:03}", elapsed.as_secs(), elapsed.subsec_nanos() / 1_000_000);

        eprintln!("best: {} {} {}[s]", cur_turn, best.score, elapsed);
        let best = best.actions;

        // eprintln!("best: {} {}", cur_turn, cur[0].score);
        // let mut best = cur[0].actions.clone();
        // best.push(cur[0].last_action);

        self.replay = best.iter().map(|s| action::Action::from(*s)).collect();
    }

    pub fn calc_rensa_plan_first(&mut self, cur_turn: usize, player: &player::Player) {
        let mut next = Vec::new();
        let mut cur = vec![BeamState::new(player.clone(), 0, Vec::new())];
        let max_fire_turn = 12;
        let beam_width = 100 * 3 * 3 * 3 * 1;
        let actions = action::Action::all_actions();
        // let allow_dead_line = Self::is_dangerous(&player.board);

        let fire_actions = [
            (self.packs[cur_turn + max_fire_turn - 3].clone(), action::Action::PutBlock { pos: 0, rot: 0, }),
            (self.packs[cur_turn + max_fire_turn - 2].clone(), action::Action::PutBlock { pos: 4, rot: 0, }),
            (self.packs[cur_turn + max_fire_turn - 1].clone(), action::Action::PutBlock { pos: 8, rot: 0, }),
        ];

        let mut best = BeamState::new(player.clone(), 0, Vec::new());
        for search_turn in 0..max_fire_turn {
            let turn = cur_turn + search_turn;
            let pack = self.packs[turn].clone();
            cur.iter().for_each(|b| {
                actions.iter().for_each(|a| {
                    if let action::Action::UseSkill = a {
                        return;
                        // if !b.player.can_use_skill() {
                        //     return;
                        // }
                    }

                    let mut player = b.player.clone();
                    let result = player.put(&pack, a);
                    if player.board.is_dead() {
                        return;
                    }

                    let mut actions = b.actions.clone();
                    actions.push(a.into());

                    let score = self.calc_score(&result, &player.board, search_turn);
                    if best.score > score {
                        best = BeamState::new(player.clone(), score, actions.clone());
                    }

                    if result.chains >= 3 {
                        return;
                    }

                    // obstacleが降ってくると危ないので
                    // if !allow_dead_line && Self::is_dangerous(&board) {
                    //     return;
                    // }

                    let min_score = fire_actions.iter().map(|a| {
                        let mut rensa_eval_board = player.clone();
                        // let result = rensa_eval_board.put(&fall, &fire_action);
                        let result = rensa_eval_board.put(&a.0, &a.1);
                        self.calc_score(&result, &player.board, search_turn)
                    }).min().unwrap();

                    next.push(BeamState::new(player.clone(), min_score, actions.clone()));
                });
            });
            next.sort();
            next.resize(beam_width, Default::default());
            std::mem::swap(&mut cur, &mut next);
            next.clear();
        }
        eprintln!("best: {} {}", cur_turn, best.score);
        let best = best.actions;

        // eprintln!("best: {} {}", cur_turn, cur[0].score);
        // let mut best = cur[0].actions.clone();
        // best.push(cur[0].last_action);

        self.replay = best.iter().map(|s| action::Action::from(*s)).collect();
    }

    pub fn replay(&mut self) -> action::Action {
        // let pos = (self.rand.next() % 9) as usize;
        // let rot = (self.rand.next() % 4) as usize;
        // action::Action::PutBlock{ pos, rot }
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


