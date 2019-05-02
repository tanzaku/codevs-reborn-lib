

use std::str::FromStr;
use std::io::Read;
use std::io::StdinLock;

use rayon::prelude::*;

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
    maybe_bommer: bool,
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
            maybe_bommer: false,
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

        if self.cur_turn == 10 && self.enemy.skill_guage >= 30 {
            self.maybe_bommer = true;
        }

        // if self.enemy.skill_guage >= 50 && self.cur_turn > 15 {
        //     self.mode.push(BestAiMode::ModeBommerKiller);
        //     return None;
        // }

        // if self.replay_player.len() > 1 {
        //     let fire_timing = self.fire_timing();
        //     if fire_timing.is_some() {
        //         self.replay_player.clear();
        //         return fire_timing;
        //     }
        // }

        if self.maybe_bommer {
            if self.cur_turn == 10 || !self.replay_player.can_replay(&self.player) {
                // let max_turn = if self.cur_turn <= 10 { 8 } else { 16 };
                let max_turn = if self.cur_turn <= 10 { 8 } else { 11 };
                // let max_turn = 8;
                let mut think_time_in_milli = 15000;
                let limit = 200;
                let mut enemy_send_obstacles = vec![0; max_turn];

                if self.rest_time_in_milli < 30 * 1000 {
                    think_time_in_milli = 1000;
                }

                let replay = self.replay_player.get_actions();
                let states = self.search(self.player.clone(), max_turn, think_time_in_milli, enemy_send_obstacles, replay);
                let best = self.get_best(self.player.clone(), limit, states);
                if let Some(best) = best {
                    self.replay_player = best;
                    eprintln!("think done bommer: {} {} {}", self.cur_turn, self.replay_player.get_actions().len(), self.replay_player.get_obstacles().last().unwrap());
                } else {
                    return Self::resign();
                }
            }
        } else {
            if !self.replay_player.can_replay(&self.player) {
                let max_turn = if self.cur_turn <= 10 { 15 } else { 13 };
                // let max_turn = if self.cur_turn <= 10 { 15 } else { 15 };
                // let max_turn = if self.cur_turn <= 10 { 15 } else { 10 };
                // let mut think_time_in_milli = if self.cur_turn <= 10 { 18000 } else { 15000 };
                let mut think_time_in_milli = 5000 * 2;
                // let limit = if self.cur_turn <= 10 { 60 } else { 30 };
                let limit = 60;
                // let mut think_time_in_milli = 5000;
                let mut enemy_send_obstacles = vec![0; max_turn];

                // emergency
                if self.rest_time_in_milli < 30 * 1000 {
                    think_time_in_milli = 1000;
                }

                let replay = self.replay_player.get_actions();
                // let replay = vec![];
                let states = self.search(self.player.clone(), max_turn, think_time_in_milli, enemy_send_obstacles, replay);
                let best = self.get_best(self.player.clone(), limit, states);
                // let best = self.get_best(self.player.clone(), 40, states);
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

            if self.rest_time_in_milli >= 30 * 1000 {
                if let Some(r) = self.fire_timing() {
                    self.replay_player = r;
                }
            }
        }

        // self.gyoushi();
        // self.extend();

        self.replay_player.replay()
    }

    fn search(&mut self, player: player::Player, max_turn: usize, think_time_in_milli: u64, enemy_send_obstacles: Vec<i32>, replay: Vec<action::Action>) -> Vec<(rensa_plan::BeamState, action::ActionResult)> {
        // let mut enemy_send_obstacles = vec![0; max_turn];

        let weights = [
                        (50000,10000,1000,300),
                        // (50000,40000,1000,1000),
                        // (50000,60000,10000,10000),
                    ];

        let context = rensa_plan::PlanContext {
            plan_start_turn: self.cur_turn,
            max_turn,
            think_time_in_milli: think_time_in_milli,
            player,
            enemy_send_obstacles,
            packs: self.packs.clone(),
            stop_search_if_3_chains: true,
            replay,
            verbose: true,
        };

        let w = weights[0];
        rensa_plan::calc_rensa_plan(&context, &mut self.rand, |result, second_chains, player, feature| {
            let obstacle_score = std::cmp::min(result.obstacle, 200);
            let feature_score = feature.keima * w.0
                                + feature.tate * w.1
                                + feature.keima2 * w.2
                                + feature.tate2 * w.3
                                ;
            obstacle_score as i64 * 50000000000 + second_chains as i64 * 500000000 + feature_score as i64
        })
    }

    fn get_best(&self, player: player::Player, limit_obstacle: i32, states: Vec<(rensa_plan::BeamState, action::ActionResult)>) -> Option<replay::Replay> {
        let mut max = -1;
        let mut choosed = None;
        states.iter().for_each(|s| {
            let val = std::cmp::min(limit_obstacle, s.1.obstacle);
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

    fn to_replay(&self, player: &player::Player, state: &rensa_plan::BeamState) -> Option<replay::Replay> {
        let actions = state.get_actions();

        if actions.is_empty() {
            None
        } else {
            let last_turn = self.cur_turn + actions.len();
            let packs = &self.packs[self.cur_turn..last_turn];
            let mut replay = replay::Replay::new();
            replay.init(&player, packs, &actions);
            Some(replay)
        }
    }

    fn fire(&mut self, player: &player::Player) -> (action::Action, i32) {
        let actions = action::Action::all_actions();
        let pack = self.packs[self.cur_turn];
        actions.par_iter().map(|a| {
            if &action::Action::UseSkill == a && !player.can_use_skill() {
                return (action::Action::UseSkill, 0);
            }

            let mut player = player.clone();
            (a.clone(), player.put(&pack, a).obstacle)
        }).max_by_key(|x| x.1).unwrap()
    }

    fn search_max_obstacles(&mut self, player: &player::Player, think_time_in_milli: u64, fall_obstacles: Vec<i32>) -> Option<replay::Replay> {
        let s = self.search(player.clone(), 10, think_time_in_milli, fall_obstacles, vec![]);
        self.get_best(player.clone(), 60, s)
    }

    // 発火して潰せるなら潰す
    fn fire_timing(&mut self) -> Option<replay::Replay> {
        if self.cur_turn % 3 != 2 {
            return None;
        }

        let enemy = self.enemy.clone();
        let cur_enemy_replay = self.search_max_obstacles(&enemy, 500 * 3, vec![]);
        if cur_enemy_replay.is_none() {
            return None;
        }
        let cur_enemy_replay = cur_enemy_replay.unwrap();
        let estimate_enemy = cur_enemy_replay.get_obstacles();
        if *cur_enemy_replay.get_obstacles().last().unwrap() < 40 {
            return None;
        }

        let mut best = self.simulate(self.replay_player.clone(), cur_enemy_replay.clone());
        let mut best_replay: replay::Replay = self.replay_player.clone();
        if best >= 0 {
            let mut min = 19;
            let player = self.player.clone();
            let fire = self.search(player.clone(), 10, 1500, vec![], vec![]);
            fire.iter().for_each(|f| {
                //  Vec<(rensa_plan::BeamState, action::ActionResult)> {
                if min >= f.1.obstacle {
                    return;
                }
                min = f.1.obstacle;
                let replay = self.to_replay(&self.player, &f.0);
                if replay.is_none() {
                    return;
                }
                let replay = replay.unwrap();
                let val = self.simulate(replay.clone(), cur_enemy_replay.clone());
                if best > val {
                    best = val;
                    best_replay = replay;
                }
            });

            eprintln!("new replay: {} {} {} {:?}", self.cur_turn, best_replay.get_obstacles().len(), best_replay.get_obstacles().last().unwrap(), estimate_enemy);
        }
        Some(best_replay)
    }

    fn simulate(&mut self, mut replay_player: replay::Replay, mut replay_enemy: replay::Replay) -> i32 {
        let mut player = self.player.clone();
        let mut enemy = self.enemy.clone();

        let mut turn = self.cur_turn;
        while replay_player.can_replay(&player) && replay_enemy.can_replay(&enemy) {
            let a1 = replay_player.replay().unwrap();
            let a2 = replay_enemy.replay().unwrap();
            let r1 = player.put(&self.packs[turn], &a1);
            let r2 = enemy.put(&self.packs[turn], &a2);
            let min = std::cmp::min(player.obstacle, enemy.obstacle);
            player.obstacle -= min;
            enemy.obstacle -= min;
            turn += 1;
        }
        // let r2 = self.search_max_obstacles(&enemy, 500 * 2, r1.get_obstacles());
        let r2 = self.search_max_obstacles(&enemy, 500 * 2, vec![]);
        if r2.is_none() {
            return -1000;
        }
        let r2 = r2.unwrap();
        let r1 = self.search_max_obstacles(&player, 500 * 2, r2.get_obstacles());
        // let r1 = self.search_max_obstacles(&player, 500 * 2, vec![]);
        if r1.is_none() {
            return 1000;
        }
        let r1 = r1.unwrap();
        let o1 = *r1.get_obstacles().last().unwrap();
        let o2 = *r2.get_obstacles().last().unwrap();
        // if o1 != o2 {
        //     return o1 - o2;
        // }
        // r1.len() as i32 - r2.len() as i32
        let score = player.obstacle + o2 - (enemy.obstacle + o1);
        // let score = o2 - o1;
        // if score < 0 {
        //     eprintln!("improve: {} {} {} {} {:?} {:?}", player.obstacle, o1, enemy.obstacle, o2, r1.get_obstacles(), r2.get_obstacles());
        // }
        score
    }

    // ボマーかカウンター狙いの場合、延長して大連鎖狙う
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

    fn extend(&mut self) {
        if self.replay_player.len() != 3 {
            return;
        }
        
        let enemy = self.enemy.clone();
        let obstacles = self.replay_player.get_obstacles();
        let cur_enemy = self.search_max_obstacles(&enemy, 1000, obstacles.clone());

        let cur_enemy = cur_enemy.map(|replay| *replay.get_obstacles().last().unwrap()).unwrap_or_default();

        if cur_enemy > obstacles.iter().sum() {
            let replay = self.replay_player.get_actions();

            let states = self.search(enemy.clone(), 3, 1000, vec![], vec![]);
            let enemy_replay = self.get_best(enemy.clone(), 40, states).unwrap();
            let enemy_send_obstacles = enemy_replay.get_obstacles();

            let states = self.search(self.player.clone(), 7, 2000, enemy_send_obstacles.clone(), replay);
            let best = self.get_best(self.player.clone(), 200, states);
            if let Some(best) = best {
                self.replay_player = best;
                eprintln!("extend: {} {} {} {:?}", self.cur_turn, self.replay_player.get_actions().len(), self.replay_player.get_obstacles().last().unwrap(), enemy_send_obstacles);
            }
        }
    }

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
            let mut think_time_in_milli = 1000;
            let enemy_send_obstacles = vec![0; max_turn];

            if self.rest_time_in_milli < 30 * 1000 {
                think_time_in_milli = 1000;
            }

            let context = rensa_plan::PlanContext {
                plan_start_turn: self.cur_turn,
                max_turn: max_turn,
                think_time_in_milli,
                player: self.player.clone(),
                enemy_send_obstacles,
                packs: self.packs.clone(),
                stop_search_if_3_chains: false,
                replay: vec![],
                verbose: true,
            };

            let states = rensa_plan::calc_rensa_plan(&context, &mut self.rand, |result, second_chains, player, feature| {
                // result.skill_guage * 10000 + result.chains as i32 * 10 - search_turn as i32
                (player.decrease_skill_guage as i64 * 10000 + second_chains as i64 + result.chains as i64 * 10) * 256
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
        //     think_time_in_milli: 1,
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
