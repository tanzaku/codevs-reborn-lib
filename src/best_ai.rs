

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

pub struct BestAi<'a> {
    cur_turn: usize,

    stdin_lock: StdinLock<'a>,
    packs: Vec<[[u8; 2]; 2]>,
    rest_time_in_milli: usize,
    player: player::Player,
    enemy: player::Player,
    rand: rand::XorShiftL,

    maybe_bommer: bool,
    current_best: replay::Replay,
}

impl<'a> BestAi<'a> {
    pub fn new(lock: StdinLock<'a>) -> Self {
        Self {
            cur_turn: 0,

            stdin_lock: lock,
            packs: Vec::new(),
            rest_time_in_milli: 0,
            player: player::Player::new(board::Board::new(), 0, 0),
            enemy: player::Player::new(board::Board::new(), 0, 0),
            rand: rand::XorShiftL::new(),

            maybe_bommer: false,
            current_best: replay::Replay::new(),
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

        // for bommer
        if self.cur_turn == 10 && self.enemy.skill_guage >= 30 {
            self.maybe_bommer = true;
        }

        if self.maybe_bommer {
            return self.kill_bommer();
        }

        if false {
        } else if self.do_counter() {
        } else if self.do_anti_counter() {
        } else if self.rensa() {
        }
        // self.snipe_enemy();

        // if self.current_best.len() == 1 {
        //     self.anti_counter();
        // }


        if self.current_best.is_empty() {
            Self::resign()
        } else {
            self.current_best.replay().unwrap()
        }
    }

    fn rensa_extend(&mut self, max_turn: usize, think_time_in_milli: u64) {
        // TODO 200 limit is too low?
        let limit = 200;
        let enemy_send_obstacles = vec![];

        let states = self.search_rensa(self.player.clone(), max_turn, think_time_in_milli, &enemy_send_obstacles);

        let best = self.get_best(self.player.clone(), limit, &enemy_send_obstacles, &states);
        if let Some(best) = best {
            self.current_best = best;
            let fire = states.iter().map(|r| r.get_chains()).collect::<Vec<_>>();
            eprintln!("extend done: {} {} {:?}", self.cur_turn, self.current_best.get_actions().len(), fire);
        }
    }

    fn rensa(&mut self) -> bool {
        if self.current_best.can_replay(&self.player, &[]) {
            return false;
        }

        let max_turn = if self.cur_turn <= 10 { 13 } else { 10 };
        let mut think_time_in_milli = if self.cur_turn <= 10 { 18000 } else { 15000 };
        let limit = 60;
        let enemy_send_obstacles = vec![];

        if self.rest_time_in_milli < 30 * 1000 {
            // emergency
            think_time_in_milli = 1000;
        }

        let states = self.search_rensa(self.player.clone(), max_turn, think_time_in_milli, &enemy_send_obstacles);

        let best = self.get_best(self.player.clone(), limit, &enemy_send_obstacles, &states);
        if let Some(best) = best {
            self.current_best = best;
            let fire = states.iter().map(|r| r.get_chains()).collect::<Vec<_>>();
            eprintln!("think done: {} {} {:?}", self.cur_turn, self.current_best.get_actions().len(), fire);
        }
        true
    }

    fn do_counter(&mut self) -> bool {
        if self.rest_time_in_milli < 30 * 1000 {
            return false;
        }
        let enemy_attack = self.fire(&self.enemy);
        if enemy_attack.2 < 40 {
            return false
        }
        let self_counter_states = self.search_rensa(self.player.clone(), 10, 15000, &[enemy_attack.2]);
        if let Some(best_counter) = self.get_best(self.player.clone(), enemy_attack.2 * 3 / 2, &[enemy_attack.2], &self_counter_states) {
            self.current_best = best_counter;
            let fire = self_counter_states.iter().map(|r| r.get_chains()).collect::<Vec<_>>();
            eprintln!("counter done: {} {} {:?}", self.cur_turn, self.current_best.get_actions().len(), fire);
        }
        true
    }

    fn do_anti_counter(&mut self) -> bool {
        if self.rest_time_in_milli < 30 * 1000 {
            return false;
        }
        if self.current_best.len() != 1 {
            return false;
        }

        let my_attack = self.fire(&self.player);
        let enemy_counter_states = self.search_rensa(self.enemy.clone(), 7, 5000, &[my_attack.2]);
        if let Some(enemy_counter_best) = self.get_best(self.enemy.clone(), 200, &[my_attack.2], &enemy_counter_states) {
            if enemy_counter_best.get_chains() >= my_attack.1.chains + 1 {
            // if enemy_counter_best.get_chains() >= my_attack.1.chains {
                self.rensa_extend(8, 13000);
            }
        }
        true
    }

    // fn anti_counter_clever(&mut self) {
    //     let enemy_attack = self.fire(&self.enemy);
    //     if enemy_attack.1.chains >= 11 {
    //         let self_counter_states = self.search_rensa(self.player.clone(), 13, 18000, &[enemy_attack.1.obstacle]);
    //         if let Some(best_counter) = self.get_best(self.player.clone(), enemy_attack.1.obstacle * 3 / 2, &[enemy_attack.1.obstacle], &self_counter_states) {
    //             self.current_best = best_counter;
    //             let fire = self_counter_states.iter().map(|r| r.get_chains()).collect::<Vec<_>>();
    //             eprintln!("counter done: {} {} {:?}", self.cur_turn, self.current_best.get_actions().len(), fire);
    //         }
    //     }

    //     // let my_attack = self.fire(&self.player);
    //     // let enemy_counter_states = self.search_rensa(self.enemy.clone(), 7, 5000, &[my_attack.1.obstacle]);
    //     // if let Some(enemy_counter_best) = self.get_best(self.enemy.clone(), 200, &[my_attack.1.obstacle], &enemy_counter_states) {
    //     //     if enemy_counter_best.get_chains() >= my_attack.1.chains + 1 {
    //     //     // if enemy_counter_best.get_chains() >= my_attack.1.chains {
    //     //         self.rensa_extend(8, 13000);
    //     //     }
    //     // }
    // }

    fn is_enemy_tactics_counter(&self) -> bool {
        let (_, _, (x,y)) = self.enemy.board.calc_max_rensa_by_erase_outer_block();
        // eprintln!("anticounter: {} {}", y, self.enemy.board.adjust_height_min(x));
        // unreachable!();
        let dy = y as i32 - self.enemy.board.adjust_height_min(x) as i32;
        dy >= 5 && dy <= 8
    }

    fn anti_counter(&mut self) {
        if self.cur_turn > 15 || !self.is_enemy_tactics_counter() {
            return;
        }

        self.current_best.clear();
        let max_turn = 8;
        let mut think_time_in_milli = if self.cur_turn <= 10 { 18000 } else { 15000 };
        let limit = 200;
        let enemy_send_obstacles = vec![];

        if self.rest_time_in_milli < 30 * 1000 {
            // emergency
            think_time_in_milli = 1000;
        }

        let states = self.search_rensa(self.player.clone(), max_turn, think_time_in_milli, &enemy_send_obstacles);

        let best = self.get_best(self.player.clone(), limit, &enemy_send_obstacles, &states);
        if let Some(best) = best {
            self.current_best = best;
            let fire = states.iter().map(|r| r.get_chains()).collect::<Vec<_>>();
            eprintln!("anti_counter done: {} {} {:?}", self.cur_turn, self.current_best.get_actions().len(), fire);
        }
    }

    fn kill_bommer(&mut self) -> action::Action {
        if self.cur_turn != 10 && self.current_best.can_replay(&self.player, &[]) {
            return self.current_best.replay().unwrap();
        }

        let max_turn = if self.cur_turn <= 10 { 8 } else { 11 };
        let mut think_time_in_milli = 15000;
        let limit = 200;
        let enemy_send_obstacles = vec![0; max_turn];

        if self.rest_time_in_milli < 30 * 1000 {
            think_time_in_milli = 1000;
        }

        let states = self.search_rensa(self.player.clone(), max_turn, think_time_in_milli, &enemy_send_obstacles);
        let best = self.get_best(self.player.clone(), limit, &enemy_send_obstacles, &states);
        if let Some(best) = best {
            self.current_best = best;
            eprintln!("think done bommer: {} {} {}", self.cur_turn, self.current_best.get_actions().len(), self.current_best.get_obstacles(&self.player).last().unwrap());
            self.current_best.replay().unwrap()
        } else {
            Self::resign()
        }
    }

    fn search_rensa(&mut self, player: player::Player, max_turn: usize, think_time_in_milli: u64, enemy_send_obstacles: &[i32]) -> Vec<replay::Replay> {
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
            let feature_score =
                                (result.fire_height as i32) * 1000
                                + feature.keima * 50
                                + feature.tate * 40
                                + feature.keima2 * 1
                                + feature.tate2 * 1
                                + feature.num_block * 2000
                                // + feature.num_block * 10000
                                // + feature.num_block * 100000
                                ;
            obstacle_score as i64 * 1000000 + feature_score as i64
        })
    }

    fn snipe_enemy(&mut self) {
        if self.current_best.is_empty() || self.current_best.len() >= 8 {
            return;
        }

        let moves = self.search_rensa(self.player.clone(), self.current_best.len(), 1000, &[]);
        self.update_best_move(&moves);
        
        let snipe_move = self.search_snipe_move(&moves);
        if snipe_move.is_empty() {
            return;
        }
        let snipe_result = self.snipe(&snipe_move);
        // if snipe_move.get_chains() > snipe_result.get_chains() {
        if snipe_result.len() >= 11 || snipe_result.get_chains() <= 8 {
            // eprintln!("update snipe: {} {} {} {} {}", self.cur_turn, self.current_best.len(), self.current_best.get_chains(), snipe_move.len(), snipe_move.get_chains());
            eprintln!("update snipe: {} {} {} {} {}", self.cur_turn, snipe_move.len(), snipe_move.get_chains(), snipe_result.len(), snipe_result.get_result().obstacle);
            self.current_best = snipe_move;
        }
    }

    fn update_best_move(&mut self, moves: &[replay::Replay]) {
        // TODO 相手のフィールドの埋まり具合に合わせて狙いを変える?
        moves.iter().for_each(|m| {
            // 構造体側の比較に任せるべきではない
            let cur_chains = std::cmp::min(13, self.current_best.get_chains());
            let chains = std::cmp::min(13, m.get_chains());
            // if cur_chains < chains || cur_chains == chains && self.current_best.len() > m.len() {
            if cur_chains <= chains && self.current_best.len() > m.len() {
                eprintln!("update_best_move: {} {} {} {} {}", self.cur_turn, self.current_best.len(), self.current_best.get_chains(), m.len(), m.get_chains());
                self.current_best = m.clone();
            }
        });
    }

    fn search_snipe_move(&mut self, moves: &[replay::Replay]) -> replay::Replay {
        // TODO 次点を探索し、それで潰せるか
        let mut second: replay::Replay = replay::Replay::new();
        let best_chains = self.current_best.get_chains();
        moves.iter().for_each(|m| {
            // 構造体側の比較に任せるべきではない
            let chains = m.get_chains();
            if chains >= best_chains || chains >= best_chains && self.current_best.len() <= m.len() {
                return;
            }
            if best_chains != chains + 1 {
                return;
            }
            // eprintln!("snipe candidate: {}", chains);
            let second_chains = second.get_chains();
            if second_chains < chains {
                second = m.clone();
            }
        });
        // eprintln!("snipe: {} {} {} {}", self.current_best.len(), best_chains, second.len(), second.get_chains());
        second
    }

    fn snipe(&mut self, snipe_move: &replay::Replay) -> replay::Replay {
        // let obstacles = snipe_move.get_raw_obstacles();
        let obstacles = snipe_move.get_obstacles(&self.player);
        self.search_rensa(self.enemy.clone(), 13, 2000, &obstacles).into_iter().max_by(|a, b| {
            a.get_chains().cmp(&b.get_chains()).then(b.len().cmp(&a.len()))
        }).unwrap()
    }

    fn get_best(&self, player: player::Player, limit_obstacle: i32, enemy_send_obstacles: &[i32], states: &[replay::Replay]) -> Option<replay::Replay> {
        let mut max = -1;
        let mut choosed = None;
        states.iter().for_each(|s| {
            let val = std::cmp::min(limit_obstacle, s.get_obstacle());
            if max < val {
                max = val;
                choosed = Some(s);
            }
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

    // fn to_replay(&self, state: &rensa_plan::SearchResult) -> Option<replay::Replay> {
    //     let actions = state.get_actions();

    //     if actions.is_empty() {
    //         None
    //     } else {
    //         let last_turn = state.fire_turn;
    //         let packs = &self.packs[self.cur_turn..last_turn];
    //         let mut replay = replay::Replay::new();
    //         replay.init(&player, packs, enemy_send_obstacles, &actions);
    //         Some(replay)
    //     }
    // }

    // fn to_replay(&self, player: &player::Player, enemy_send_obstacles: &[i32], state: &replay::Replay) -> Option<replay::Replay> {
    //     let actions = state.get_actions();

    //     if actions.is_empty() {
    //         None
    //     } else {
    //         let last_turn = self.cur_turn + actions.len();
    //         let packs = &self.packs[self.cur_turn..last_turn];
    //         let mut replay = replay::Replay::new();
    //         replay.init(&player, packs, enemy_send_obstacles, &actions);
    //         Some(replay)
    //     }
    // }

    fn fire(&self, player: &player::Player) -> (action::Action, action::ActionResult, i32) {
        let actions = action::Action::all_actions();
        let pack = self.packs[self.cur_turn];
        actions.par_iter().map(|a| {
            if &action::Action::UseSkill == a && !player.can_use_skill() {
                return (action::Action::UseSkill, Default::default(), 0);
            }

            let mut player = player.clone();
            let result = player.put(&pack, a);
            (a.clone(), result.clone(), result.obstacle - player.obstacle)
        }).max_by_key(|x| x.1.obstacle).unwrap()
    }

    // fn search_max_obstacles(&mut self, player: &player::Player, think_time_in_milli: u64, fall_obstacles: Vec<i32>) -> Option<replay::Replay> {
    //     let s = self.search_rensa(player.clone(), 10, think_time_in_milli, &fall_obstacles);
    //     self.get_best(player.clone(), 60, &fall_obstacles, s)
    // }

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

    //     let mut best = self.simulate(self.current_best.clone(), cur_enemy_replay.clone());
    //     let mut best_replay: replay::Replay = self.current_best.clone();
    //     if best >= 0 {
    //         let mut min = 29;
    //         let player = self.player.clone();
    //         let fire = self.search_rensa(player.clone(), 10, 1000, &vec![], vec![]);
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

    // fn simulate(&mut self, mut current_best: replay::Replay, mut replay_enemy: replay::Replay) -> i32 {
    //     let mut player = self.player.clone();
    //     let mut enemy = self.enemy.clone();

    //     let mut turn = self.cur_turn;
    //     while current_best.can_replay(&player, &[]) && replay_enemy.can_replay(&enemy, &[]) {
    //         let a1 = current_best.replay().unwrap();
    //         let a2 = replay_enemy.replay().unwrap();
    //         let _r1 = player.put(&self.packs[turn], &a1);
    //         let _r2 = enemy.put(&self.packs[turn], &a2);
    //         let min = std::cmp::min(player.obstacle, enemy.obstacle);
    //         player.obstacle -= min;
    //         enemy.obstacle -= min;
    //         turn += 1;
    //     }
    //     let r2 = self.search_max_obstacles(&enemy, 500 * 3, vec![]);
    //     if r2.is_none() {
    //         return -1000;
    //     }
    //     let r2 = r2.unwrap();
    //     // let r1 = self.search_max_obstacles(&player, 500 * 2, r2.get_obstacles(&enemy));
    //     // if r1.is_none() {
    //     //     return 1000;
    //     // }
    //     // let r1 = r1.unwrap();
    //     // let o1 = r1.get_obstacles_score(&player);
    //     let o2 = r2.get_obstacles_score(&enemy);
    //     // let score = o2 - o1;
    //     let score = o2;
    //     // eprintln!("improve: {} {} {} {} {:?} {:?}", player.obstacle, o1, enemy.obstacle, o2, r1.get_obstacles(&player), r2.get_obstacles(&enemy));
    //     eprintln!("improve: {} {} {:?}", enemy.obstacle, o2, r2.get_obstacles(&enemy));
    //     score
    // }

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
