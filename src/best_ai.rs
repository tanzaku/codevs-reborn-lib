

use std::str::FromStr;
use std::io::Read;
use std::io::StdinLock;

use super::action;
use super::board;
use super::player;
use super::rensa_plan;
use super::skill_plan;
use super::replay;
use super::consts::{W,H,MAX_TURN};

use super::rand;

#[derive(Eq, PartialEq)]
enum BestAiMode {
    ModeRensa,
    ModeBommer,
    ModeBommerKiller,
}

pub struct BestAi<'a> {
    cur_turn: usize,

    stdin_lock: StdinLock<'a>,
    packs: Vec<[[u8; 2]; 2]>,
    rest_time_in_milli: usize,
    prev_obstacle_stock: i32,
    player: player::Player,
    enemy: player::Player,
    mode: Vec<BestAiMode>,
    rand: rand::XorShiftL,

    recalc_turn: usize,
    replay_player: replay::Replay,
    replay_enemy: replay::Replay,
}

impl<'a> BestAi<'a> {
    pub fn new(lock: StdinLock<'a>) -> Self {
        Self {
            cur_turn: 0,

            stdin_lock: lock,
            packs: Vec::new(),
            rest_time_in_milli: 0,
            prev_obstacle_stock: 0,
            player: player::Player::new(board::Board::new(), 0, 0),
            enemy: player::Player::new(board::Board::new(), 0, 0),
            mode: vec![BestAiMode::ModeRensa],
            rand: rand::XorShiftL::new(),

            recalc_turn: 0,
            replay_player: replay::Replay::new(),
            replay_enemy: replay::Replay::new(),
        }
    }

    fn read1<T: FromStr>(&mut self) -> T {
        let token = self.stdin_lock.by_ref().bytes().map(|c| c.unwrap() as char)
            .skip_while(|c| c.is_whitespace())
            .take_while(|c| !c.is_whitespace())
            .collect::<String>();
        token.parse::<T>().ok().unwrap()
    }

    fn read_game_input(&mut self) {
        (0..MAX_TURN).for_each(|_| {
            let v1 = self.read1();
            let v2 = self.read1();
            let v3 = self.read1();
            let v4 = self.read1();
            self.read1::<String>();
            self.packs.push([[v1, v2], [v3, v4]]);
        });
    }

    fn read_board(&mut self) -> board::Board {
        let mut board = [0; (W * H) as usize];
        (0..W*H).for_each(|p| { board[p as usize] = self.read1::<u8>(); });
        board::Board::from_board(board)
    }

    fn read_turn_input(&mut self) {
        self.prev_obstacle_stock = self.player.obstacle;

        self.cur_turn = self.read1();
        // eprintln!("start read {}", self.cur_turn);
        self.rest_time_in_milli = self.read1::<usize>();
        self.player.obstacle = self.read1();
        self.player.skill_guage = self.read1();
        let _player_score = self.read1::<u32>();
        self.player.board = self.read_board();
        self.read1::<String>();

        let _rest_time_in_milli = self.read1::<u32>();
        self.enemy.obstacle = self.read1();
        self.enemy.skill_guage = self.read1();
        let _enemy_score = self.read1::<u32>();
        self.enemy.board = self.read_board();
        self.read1::<String>();
    }

    pub fn exec(&mut self) {
        println!("test-best-ai");
        self.read_game_input();
        loop {
            self.read_turn_input();
            let act = self.think();
            println!("{}", act);
        }
    }

    fn think(&mut self) -> action::Action {
        loop {
            let result = match self.mode.last().unwrap() {
                &BestAiMode::ModeBommerKiller => self.kill_bommer(),
                &BestAiMode::ModeRensa => self.rensa(),
                &BestAiMode::ModeBommer => self.bommer(),
            };

            if let Some(a) = result {
                return a;
            }
            
            self.replay_player.clear();
        }
    }

    fn rensa(&mut self) -> Option<action::Action> {
        if self.should_bombed() {
            self.mode.push(BestAiMode::ModeBommer);
            return None;
        }

        if self.enemy.skill_guage >= 50 && self.cur_turn > 15 {
            self.mode.push(BestAiMode::ModeBommerKiller);
            return None;
        }

        // if self.cur_turn < 4 && self.cur_turn % 2 == 0 {
        //     let max_turn = if self.replay_player.len() == 0 { 15 - self.cur_turn } else { self.replay_player.len() };
        //     let think_time_in_sec = if self.cur_turn == 2 { 15 } else { 5 };
        //     let enemy_send_obstacles = vec![0; max_turn];

        //     let replay = self.replay_player.get_actions();
        //     let best =
        //         if self.cur_turn < 2 {
        //             self.search_best(self.player.clone(), max_turn, think_time_in_sec, enemy_send_obstacles, replay, true)
        //         } else {
        //             self.search_best2(self.player.clone(), max_turn, think_time_in_sec, enemy_send_obstacles, replay, true)
        //         };
        //     if let Some(best) = best {
        //         self.replay_player = best;
        //         // eprintln!("rensa: {} {} {}", self.cur_turn, s.0.actions.len(), s.1.chains);
        //         eprintln!("think done: {} {} {}", self.cur_turn, self.replay_player.get_actions().len(), self.replay_player.get_obstacles().last().unwrap());
        //     } else {
        //         return Self::resign();
        //     }
            
        // } else 
        if !self.replay_player.can_replay(&self.player) {
            let max_turn = if self.cur_turn <= 10 { 15 } else { 13 };
            let mut think_time_in_sec = if self.cur_turn <= 10 { 18 } else { 15 };
            // let mut think_time_in_sec = if self.cur_turn == 0 { 10 } else { 1 };
            // let mut think_time_in_sec = if self.cur_turn == 0 { 18 } else { 1 };
            let mut enemy_send_obstacles = vec![0; max_turn];
            // if max_turn > 12 {
            //     enemy_send_obstacles[12] = 0;
            // }

            // emergency
            if self.rest_time_in_milli < 30 * 1000 {
                think_time_in_sec = 1;
            }

            let replay = self.replay_player.get_actions();
            let replay = vec![];
            let best = self.search_best2(self.player.clone(), max_turn, think_time_in_sec, enemy_send_obstacles, replay, true);
            if let Some(best) = best {
                self.replay_player = best;
                // eprintln!("rensa: {} {} {}", self.cur_turn, s.0.actions.len(), s.1.chains);
                // if self.cur_turn == 0 {
                    eprintln!("think done: {} {} {}", self.cur_turn, self.replay_player.get_actions().len(), self.replay_player.get_obstacles().last().unwrap());
                // }
            } else {
                return Self::resign();
            }
        }

        // self.gyoushi();
        // self.extend();

        self.replay_player.replay()
    }

    fn gyoushi(&mut self) {
        if self.cur_turn != 2 {
            return;
        }

        let max_turn = std::cmp::min(self.replay_player.len() + 3, 15);
        let think_time_in_sec = 5;
        let send_obstacles = self.replay_player.get_obstacles();
        // send_obstacles.resize(max_turn, 0);
        let result = self.search_best(self.enemy.clone(), max_turn, think_time_in_sec, send_obstacles, vec![], false);
        if result.is_none() {
            return;
        }

        let replay = result.unwrap();
        if replay.len() > self.replay_player.len() {
            return;
        }
        if replay.len() == self.replay_player.len() && replay.get_obstacles().last().unwrap() <= self.replay_player.get_obstacles().last().unwrap() {
            return;
        }

        let think_time_in_sec = 10;
        let send_obstacles = replay.get_obstacles();
        let replay = self.replay_player.get_actions();
        if let Some(replay) = self.search_best(self.player.clone(), max_turn, think_time_in_sec, send_obstacles.clone(), replay, false) {
            self.replay_player = replay;
        }
        eprintln!("rensa vs: {} {:?} {:?}", self.cur_turn, send_obstacles, self.replay_player.get_obstacles());
    }

    fn search_best(&mut self, player: player::Player, max_turn: usize, think_time_in_sec: u64, enemy_send_obstacles: Vec<i32>, replay: Vec<action::Action>, verbose: bool) -> Option<replay::Replay> {
        // let mut enemy_send_obstacles = vec![0; max_turn];

        let context = rensa_plan::PlanContext {
            plan_start_turn: self.cur_turn,
            max_turn,
            think_time_in_sec,
            player: player.clone(),
            enemy_send_obstacles,
            packs: self.packs.clone(),
            stop_search_if_3_chains: true,
            replay,
            // verbose,
            verbose: false,
        };

        let states = rensa_plan::calc_rensa_plan(&context, |result, player, search_turn, feature| {
        // let states = rensa_plan::calc_rensa_plan_cand(&context, |result, player, search_turn, feature| {
            let obstacle_score = std::cmp::min(result.obstacle, 200);
            // let obstacle_score = result.obstacle;
            let h = result.fire_height as i32;
            let h2 = player.board.max_height() as i32;
                        // let pattern = player.board.calc_pattern();
                        // let pattern_score = (pattern.0 + pattern.1) as i32 * 10000;

            let feature_score = feature.keima * 50000
                                + feature.tate * 10000
                                // + feature.tate * 10000
                                + feature.keima2 * 1000
                                + feature.tate2 * 300
                                ;
            // let feature_score = feature.keima * 20000
            //                     // + feature.tate * 40000
            //                     + feature.tate * 10000
            //                     + feature.keima2 * 2000
            //                     // + feature.tate2 * 10000
            //                     + feature.tate2 * 1000
            //                     ;
            // obstacle_score * 5000000 + feature_score + (2 * h - h2) * 256 + (self.rand.next() & 0xFF) as i32
            // obstacle_score * 5000000 + feature_score + (self.rand.next() & 0xFF) as i32
            obstacle_score * 5000000 + feature_score
            // obstacle_score * 5000000 + feature_score + (2 * h - h2) * 100000 + (self.rand.next() & 0xFF) as i32
            // (obstacle_score * 100000 + (self.rand.next() & 0xFF) as i32)
        });

        let mut max = -1.0;
        // let mut max = -1;
        let mut choosed = None;
        let mut turn = 0.0;
        states.into_iter().for_each(|s| {
            // eprintln!("state: {}", s.1.obstacle);
            turn += 1.0;
            // let val = std::cmp::min(80, s.1.obstacle) as f64;
            let val = std::cmp::min(60, s.1.obstacle) as f64;
            // let val = std::cmp::min(40, s.1.obstacle) as f64;
            // let val = s.1.obstacle as f64 / turn;
            // let val = val as f64 / turn;
            // let val = s.1.obstacle as f64;
            if max < val {
                max = val;
                choosed = Some(s);
            }
        });

        let mut replay = replay::Replay::new();
        match choosed {
            None => None,
            Some(s) => {
                let actions = s.0.get_actions();

                // eprintln!("come: {}", actions.len());
                if actions.is_empty() {
                    None
                } else {
                    let last_turn = self.cur_turn + actions.len();
                    let packs = &self.packs[self.cur_turn..last_turn];
                    replay.init(&player, packs, &actions);
                    Some(replay)
                }
            }
        }
    }

    fn search_best2(&mut self, player: player::Player, max_turn: usize, think_time_in_sec: u64, enemy_send_obstacles: Vec<i32>, replay: Vec<action::Action>, verbose: bool) -> Option<replay::Replay> {
        // let mut enemy_send_obstacles = vec![0; max_turn];

        let weights = [
                        // (50000,10000,1000,300),
                        (50000,40000,1000,1000),
                        // (50000,60000,10000,10000),
                        // (50000,50000,10000,10000),
                    ];
        // let max_turn = if self.player.board.num_obstacle() as i32 + self.player.obstacle > self.enemy.board.num_obstacle() as i32 + self.enemy.obstacle {
        //                     max_turn - 3
        //                 } else {
        //                     max_turn
        //                 };
        let context = rensa_plan::PlanContext {
            plan_start_turn: self.cur_turn,
            max_turn,
            think_time_in_sec: think_time_in_sec / weights.len() as u64,
            player: player.clone(),
            enemy_send_obstacles,
            packs: self.packs.clone(),
            stop_search_if_3_chains: true,
            replay,
            // verbose,
            verbose: false,
        };

        let states = weights.iter().map(|w| {
            rensa_plan::calc_rensa_plan(&context, |result, player, search_turn, feature| {
                let obstacle_score = std::cmp::min(result.obstacle, 200);
                let feature_score = feature.keima * w.0
                                    + feature.tate * w.1
                                    + feature.keima2 * w.2
                                    + feature.tate2 * w.3
                                    ;
                // obstacle_score * 5000000 + feature_score + (self.rand.next() & 0xFF) as i32
                obstacle_score * 5000000 + feature_score
            })
        }).collect::<Vec<_>>();

        let mut max = -1;
        // let mut max = -1;
        let mut choosed = None;
        for i in 0..states.len() {
            for j in 0..context.max_turn {
                let val = std::cmp::min(60, states[i][j].1.obstacle);
                if max < val {
                    max = val;
                    choosed = Some(&states[i][j]);
                }
            }
        }

        let mut replay = replay::Replay::new();
        match choosed {
            None => None,
            Some(s) => {
                let actions = s.0.get_actions();

                // eprintln!("come: {}", actions.len());
                if actions.is_empty() {
                    None
                } else {
                    let last_turn = self.cur_turn + actions.len();
                    let packs = &self.packs[self.cur_turn..last_turn];
                    replay.init(&player, packs, &actions);
                    Some(replay)
                }
            }
        }
    }

    // fn fire_timing(&mut self) {
    //     let enemy_fall_obstacles = self.fire();
    //     if enemy_fall_obstacles == 0 {
    //         return;
    //     }

    //     let mut enemy = self.enemy.clone();
    //     self.replay_enemy = self.search(&enemy, vec![]);
    //     if self.replay_enemy.max_obstacles() == 0 {
    //         return;
    //     }

    //     let enemy_best = self.enemy_search(&enemy, enemy_fall_obstacles);
    //     if enemy_best == 0 {
    //         ;
    //     }
    // }

    // fn extend(&mut self) {
    //     if self.is_enemy_bommer() {
    //         if self.enemy_bommer() {
    //             self.extend_enemy_bommer();
    //         }
    //     } else {
    //         if self.enemy_conuter() {
    //             self.extend_enemy_counter();
    //         }
    //     }
    // }

    fn kill_bommer(&mut self) -> Option<action::Action> {
        if self.should_bombed() {
            self.mode.push(BestAiMode::ModeBommer);
            return None;
        }

        if self.enemy.skill_guage <= 20 {
            self.mode.pop();
            return None;
        }

        if self.replay_player.is_empty() {
            let max_turn = 5;
            let mut think_time_in_sec = 1;
            let enemy_send_obstacles = vec![0; max_turn];

            if self.rest_time_in_milli < 30 * 1000 {
                think_time_in_sec = 1;
            }

            let context = rensa_plan::PlanContext {
                plan_start_turn: self.cur_turn,
                max_turn: max_turn,
                think_time_in_sec,
                player: self.player.clone(),
                enemy_send_obstacles,
                packs: self.packs.clone(),
                stop_search_if_3_chains: false,
                replay: vec![],
                verbose: false,
            };

            let states = rensa_plan::calc_rensa_plan(&context, |result, player, search_turn, feature| {
                // result.skill_guage * 10000 + result.chains as i32 * 10 - search_turn as i32
                player.decrease_skill_guage * 10000 + result.chains as i32 * 10
            });

            let best = states.into_iter().max_by_key(|s| s.0.player.decrease_skill_guage).unwrap();
            let actions = best.0.get_actions();
            let last_turn = self.cur_turn + actions.len();
            let packs = &self.packs[self.cur_turn..last_turn];
            self.replay_player.init(&self.player, packs, &actions);
            // eprintln!("kill_bommer: {} {} {}", self.cur_turn, best.0.actions.len(), best.0.player.decrease_skill_guage);
        }

        if let Some(r) = self.replay_player.replay() {
            Some(r)
        } else {
            Self::resign()
        }
    }

    fn bommer(&mut self) -> Option<action::Action> {
        if self.player.skill_guage < 80 {
            self.mode.pop();
            return None;
        }

        // let max_turn = 3;
        // let context = skill_plan::PlanContext {
        //     plan_start_turn: self.cur_turn,
        //     max_turn: max_turn,
        //     think_time_in_sec: 1,
        //     player: self.player.clone(),
        //     enemy_send_obstacles: vec![0; max_turn],
        // };
        // let mut skill_plan = skill_plan::SkillPlan::new();

        // skill_plan.set_pack(self.packs.clone());
        // skill_plan.calc_skill_plan(&context);
        // let replay = if self.player.can_use_skill() { action::Action::UseSkill } else { skill_plan.replay() };
        let replay = action::Action::UseSkill;

        Some(replay)
    }

    fn should_bombed(&self) -> bool {
        if self.player.skill_guage < 80 {
            return false;
        }

        let mut b = self.player.board.clone();
        let result = b.use_skill();
        result.obstacle >= 50
    }

    fn resign() -> Option<action::Action> {
        Some(action::Action::PutBlock { pos: 0, rot: 0, })
    }
}
