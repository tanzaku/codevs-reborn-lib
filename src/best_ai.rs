

use std::str::FromStr;
use std::io::Read;
use std::io::StdinLock;

use rayon::prelude::*;

use super::action;
use super::board;
use super::player;
use super::rensa_plan;
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
        if self.should_bombed() {
            return action::Action::UseSkill;
        }

        if self.cur_turn == 10 && self.enemy.skill_guage >= 30 {
            self.maybe_bommer = true;
        }

        if self.maybe_bommer {
            return self.kill_bommer();
        }

        self.rensa()
    }

    fn rensa(&mut self) -> action::Action {
        self.search_kill_enemy();

        let enemy_obstacles = self.replay_enemy.get_obstacles(&self.enemy);
        if !self.replay_player.can_replay(&self.player, &enemy_obstacles) {
            let max_turn = if self.cur_turn <= 10 { 13 } else { 10 };
            let mut think_time_in_milli = if self.cur_turn <= 10 { 18000 } else { 15000 };
            let limit = 60;
            let enemy_send_obstacles = vec![];

            if self.rest_time_in_milli < 30 * 1000 {
                // emergency
                think_time_in_milli = 1000;
            }

            let replay = self.replay_player.get_actions();
            let states = self.search(self.player.clone(), max_turn, think_time_in_milli, &enemy_send_obstacles, replay);

            let best = self.get_best(self.player.clone(), limit, &enemy_send_obstacles, states);
            if let Some(best) = best {
                self.replay_player = best;
                eprintln!("think done: {} {} {:?}", self.cur_turn, self.replay_player.get_actions().len(), self.replay_player.get_obstacles(&self.player));
            }
        }

        if self.replay_player.is_empty() {
            Self::resign()
        } else {
            self.replay_player.replay().unwrap()
        }
    }

    fn kill_bommer(&mut self) -> action::Action {
        if self.cur_turn != 10 && self.replay_player.can_replay(&self.player, &[]) {
            return self.replay_player.replay().unwrap();
        }

        let max_turn = if self.cur_turn <= 10 { 8 } else { 11 };
        let mut think_time_in_milli = 15000;
        let limit = 200;
        let enemy_send_obstacles = vec![0; max_turn];

        if self.rest_time_in_milli < 30 * 1000 {
            think_time_in_milli = 1000;
        }

        let replay = self.replay_player.get_actions();
        let states = self.search(self.player.clone(), max_turn, think_time_in_milli, &enemy_send_obstacles, replay);
        let best = self.get_best(self.player.clone(), limit, &enemy_send_obstacles, states);
        if let Some(best) = best {
            self.replay_player = best;
            eprintln!("think done bommer: {} {} {}", self.cur_turn, self.replay_player.get_actions().len(), self.replay_player.get_obstacles(&self.player).last().unwrap());
            self.replay_player.replay().unwrap()
        } else {
            Self::resign()
        }
    }

    fn search(&mut self, player: player::Player, max_turn: usize, think_time_in_milli: u64, enemy_send_obstacles: &[i32], _replay: Vec<action::Action>) -> Vec<rensa_plan::SearchResult> {
        let context = rensa_plan::PlanContext {
            plan_start_turn: self.cur_turn,
            max_turn,
            think_time_in_milli: think_time_in_milli,
            player,
            enemy_send_obstacles,
            packs: &self.packs,
        };

        rensa_plan::calc_rensa_plan(&context, &mut self.rand, |result, _player, feature| {
            let obstacle_score = std::cmp::min(result.obstacle, 200);
            let feature_score = feature.keima * 50
                                + feature.tate * 40
                                + feature.keima2 * 1
                                + feature.tate2 * 1
                                + feature.num_block * 100
                                // + feature.num_block * 10000
                                // + feature.num_block * 100000
                                ;
            obstacle_score as i64 * 1000 + feature_score as i64
        })
    }

    fn get_best(&self, player: player::Player, limit_obstacle: i32, enemy_send_obstacles: &[i32], states: Vec<rensa_plan::SearchResult>) -> Option<replay::Replay> {
        let mut max = -1;
        let mut choosed = None;
        let mut turn = 0;
        let mut choosed_turn = -100;
        states.iter().for_each(|s| {
            let val = std::cmp::min(limit_obstacle, s.result.obstacle);
            if max < val {
                max = val;
                choosed = Some(s);
                choosed_turn = turn;
            }
            turn += 1;
        });

        let mut replay = replay::Replay::new();
        match choosed {
            None => None,
            Some(s) => {
                let actions = s.get_actions();

                if actions.is_empty() {
                    None
                } else {
                    let last_turn = self.cur_turn + actions.len();
                    let packs = &self.packs[self.cur_turn..last_turn];
                    replay.init(&player, packs, enemy_send_obstacles, &actions);
                    Some(replay)
                }
            }
        }
    }

    fn to_replay(&self, player: &player::Player, enemy_send_obstacles: &[i32], state: &rensa_plan::SearchResult) -> Option<replay::Replay> {
        let actions = state.get_actions();

        if actions.is_empty() {
            None
        } else {
            let last_turn = self.cur_turn + actions.len();
            let packs = &self.packs[self.cur_turn..last_turn];
            let mut replay = replay::Replay::new();
            replay.init(&player, packs, enemy_send_obstacles, &actions);
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
        let s = self.search(player.clone(), 10, think_time_in_milli, &fall_obstacles, vec![]);
        self.get_best(player.clone(), 60, &fall_obstacles, s)
    }

    // 発火して潰せるなら潰す
    // fn fire_timing(&mut self) -> Option<replay::Replay> {
    //     if self.cur_turn % 3 != 2 {
    //         return None;
    //     }

    //     let enemy = self.enemy.clone();
    //     let cur_enemy_replay = self.search_max_obstacles(&enemy, 500 * 4, vec![]);
    //     if cur_enemy_replay.is_none() {
    //         return None;
    //     }
    //     let cur_enemy_replay = cur_enemy_replay.unwrap();
    //     let estimate_enemy = cur_enemy_replay.get_obstacles(&enemy);
    //     if *estimate_enemy.last().unwrap() < 40 {
    //         return None;
    //     }

    //     let mut best = self.simulate(self.replay_player.clone(), cur_enemy_replay.clone());
    //     let mut best_replay: replay::Replay = self.replay_player.clone();
    //     if best >= 0 {
    //         let mut min = 29;
    //         let player = self.player.clone();
    //         let fire = self.search(player.clone(), 10, 1000, &vec![], vec![]);
    //         fire.iter().for_each(|f| {
    //             if min >= f.1.obstacle {
    //                 return;
    //             }
    //             min = f.1.obstacle;
    //             if min > 60 {
    //                 min = 10000;
    //             }
    //             let replay = self.to_replay(&self.player, &vec![], &f);
    //             if replay.is_none() {
    //                 return;
    //             }
    //             let replay = replay.unwrap();
    //             let val = self.simulate(replay.clone(), cur_enemy_replay.clone());
    //             if best > val {
    //                 best = val;
    //                 best_replay = replay;
    //             }
    //         });

    //         self.replay_enemy = cur_enemy_replay;
    //         eprintln!("new replay: {} {} {} {:?}", self.cur_turn, best_replay.get_obstacles(&player).len(), best_replay.get_obstacles(&player).last().unwrap(), estimate_enemy);
    //     }
    //     Some(best_replay)
    // }

    fn simulate(&mut self, mut replay_player: replay::Replay, mut replay_enemy: replay::Replay) -> i32 {
        let mut player = self.player.clone();
        let mut enemy = self.enemy.clone();

        let mut turn = self.cur_turn;
        while replay_player.can_replay(&player, &[]) && replay_enemy.can_replay(&enemy, &[]) {
            let a1 = replay_player.replay().unwrap();
            let a2 = replay_enemy.replay().unwrap();
            let _r1 = player.put(&self.packs[turn], &a1);
            let _r2 = enemy.put(&self.packs[turn], &a2);
            let min = std::cmp::min(player.obstacle, enemy.obstacle);
            player.obstacle -= min;
            enemy.obstacle -= min;
            turn += 1;
        }
        let r2 = self.search_max_obstacles(&enemy, 500 * 3, vec![]);
        if r2.is_none() {
            return -1000;
        }
        let r2 = r2.unwrap();
        // let r1 = self.search_max_obstacles(&player, 500 * 2, r2.get_obstacles(&enemy));
        // if r1.is_none() {
        //     return 1000;
        // }
        // let r1 = r1.unwrap();
        // let o1 = r1.get_obstacles_score(&player);
        let o2 = r2.get_obstacles_score(&enemy);
        // let score = o2 - o1;
        let score = o2;
        // eprintln!("improve: {} {} {} {} {:?} {:?}", player.obstacle, o1, enemy.obstacle, o2, r1.get_obstacles(&player), r2.get_obstacles(&enemy));
        eprintln!("improve: {} {} {:?}", enemy.obstacle, o2, r2.get_obstacles(&enemy));
        score
    }

    fn should_bombed(&self) -> bool {
        if !self.player.can_use_skill() {
            return false;
        }

        let mut b = self.player.board.clone();
        let result = b.use_skill();
        result.obstacle >= 50
    }

    fn resign() -> action::Action {
        action::Action::PutBlock { pos: 0, rot: 0, }
    }
}
