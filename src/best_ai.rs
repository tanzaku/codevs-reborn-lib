

use std::str::FromStr;
use std::io::Read;
use std::collections::VecDeque;

use super::action;
use super::board;
use super::player;
use super::rensa_plan;
use super::replay;
use super::consts::{W,H,MAX_TURN};

use super::rand;

pub struct BestAi<T> {
    cur_turn: usize,

    // stdin_lock: StdinLock<'a>,
    stdin_lock: T,
    packs: Vec<[[u8; 2]; 2]>,
    rest_time_in_milli: usize,
    player: player::Player,
    enemy: player::Player,
    rand: rand::XorShiftL,

    found_explicit_counter_turn: usize,
    maybe_bommer: bool,
    best_fire_enemy_history: VecDeque<i32>,
    current_best: replay::Replay,
}

impl<U> BestAi<U> where
    U: Read
{
    pub fn new(lock: U) -> Self {
        Self {
            cur_turn: 0,

            stdin_lock: lock,
            packs: Vec::new(),
            rest_time_in_milli: 0,
            player: player::Player::new(board::Board::new(), 0, 0),
            enemy: player::Player::new(board::Board::new(), 0, 0),
            rand: rand::XorShiftL::new(),

            found_explicit_counter_turn: 0,
            maybe_bommer: false,
            best_fire_enemy_history: VecDeque::new(),
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

    pub fn rensa_search_test(&mut self) -> Vec<u8> {
        self.read_game_input();
        self.read_turn_input();
        let states = self.search_rensa(self.player.clone(), 13, 18000, &[]);
        states.iter().map(|s| s.get_chains()).collect()
    }

    fn think(&mut self) -> action::Action {
        // for bommer
        if self.cur_turn == 10 && self.enemy.skill_guage >= 30 {
            self.maybe_bommer = true;
        }

        if self.maybe_bommer {
            return self.kill_bommer();
        }

        self.best_fire_enemy_history.push_back(self.fire(&self.enemy).2);
        if self.best_fire_enemy_history.len() > 5 {
            self.best_fire_enemy_history.pop_front();
        }

        if false {
        } else if self.do_counter() {
        } else if self.rensa() {
        } else if self.do_anti_counter() {
        } else if self.anti_counter_kera() {
        }

        if self.current_best.is_empty() {
            Self::resign()
        } else {
            self.current_best.replay().unwrap()
        }
    }

    fn rensa_extend(&mut self, max_turn: usize, think_time_in_milli: u64) {
        let limit = 10000;
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
        let enemy_attack = *self.best_fire_enemy_history.back().unwrap();
        let n = self.best_fire_enemy_history.len();
        let max_enemy_attack = *self.best_fire_enemy_history.iter().take(n - 1).max().unwrap_or(&0);
        let threshold = if self.cur_turn < 15 { 40 } else { 30 };
        if enemy_attack < threshold || max_enemy_attack >= enemy_attack {
            return false
        }
        let self_counter_states = self.search_rensa(self.player.clone(), 10, 15000, &[enemy_attack]);
        if let Some(best_counter) = self.get_best(self.player.clone(), enemy_attack * 3 / 2, &[enemy_attack], &self_counter_states) {
            self.current_best = best_counter;
            let fire = self_counter_states.iter().map(|r| r.get_chains()).collect::<Vec<_>>();
            eprintln!("counter done: {} {} {:?}", self.cur_turn, self.current_best.get_actions().len(), fire);
        }
        true
    }

    fn do_anti_counter(&mut self) -> bool {
        let limit = 10000;
        if self.rest_time_in_milli < 30 * 1000 {
            return false;
        }
        if self.current_best.len() != 1 {
            return false;
        }

        let my_attack = self.fire(&self.player);
        let enemy_counter_states = self.search_rensa(self.enemy.clone(), 7, 5000, &[my_attack.2]);
        if let Some(enemy_counter_best) = self.get_best(self.enemy.clone(), limit, &[my_attack.2], &enemy_counter_states) {
            if enemy_counter_best.get_chains() >= my_attack.1.chains + 1 {
                self.rensa_extend(8, 13000);
            }
        }
        true
    }

    fn enemy_counter_result(&self) -> (bool, action::ActionResult) {
        let (_, result, (x,y)) = self.enemy.board.calc_max_rensa_by_erase_block();
        // eprintln!("anticounter: {} {}", y, self.enemy.board.adjust_height_min(x));
        // unreachable!();
        let dy = y as i32 - self.enemy.board.adjust_height_min(x) as i32;
        (dy >= 5 && dy <= 8, result)
    }

    fn anti_counter_kera(&mut self) -> bool {
        let (is_counter, result) = self.enemy_counter_result();
        if !is_counter || self.current_best.get_chains() >= result.chains + 2 {
            return false;
        }
        if self.rest_time_in_milli < 30 * 1000 {
            return false;
        }

        self.found_explicit_counter_turn = self.cur_turn;
        self.current_best.clear();
        let max_turn = 8;
        let think_time_in_milli = 15000;
        let limit = 10000;
        let enemy_send_obstacles = vec![];

        let states = self.search_rensa(self.player.clone(), max_turn, think_time_in_milli, &enemy_send_obstacles);

        let best = self.get_best(self.player.clone(), limit, &enemy_send_obstacles, &states);
        if let Some(best) = best {
            self.current_best = best;
            let fire = states.iter().map(|r| r.get_chains()).collect::<Vec<_>>();
            eprintln!("anti_counter done: {} {} {:?}", self.cur_turn, self.current_best.get_actions().len(), fire);
        }

        true
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

        rensa_plan::calc_rensa_plan(&context, &mut self.rand, |result, player, feature| {
            let obstacle_score = std::cmp::min(result.obstacle, 200);
            let max_height = (std::cmp::max(H - 2, player.board.max_height()) - (H - 2)) as i32;
            // let feature_score =
            //                     feature.pairX * 30000
            //                     + feature.pair5 * 2000
            //                     + feature.num_block * 20
            //                     ;
            let feature_score =
                                (result.fire_height as i32) * 1000
                                - max_height * 10000
                                + feature.keima * 50
                                + feature.tate * 40
                                + feature.keima2 * 1
                                + feature.tate2 * 1
                                + feature.num_block * 2000
                                ;
            obstacle_score as i64 * 1000000 + feature_score as i64
        })
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

    fn fire(&self, player: &player::Player) -> (action::Action, action::ActionResult, i32) {
        let actions = action::Action::all_actions();
        let pack = self.packs[self.cur_turn];
        actions.iter().map(|a| {
            if &action::Action::UseSkill == a && !player.can_use_skill() {
                return (action::Action::UseSkill, Default::default(), 0);
            }

            let mut player = player.clone();
            let result = player.put(&pack, a);
            (a.clone(), result.clone(), -player.obstacle)
        }).max_by_key(|x| x.1.obstacle).unwrap()
    }

    fn resign() -> action::Action {
        action::Action::PutBlock { pos: 0, rot: 0, }
    }
}
