

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

    replay_player: replay::Replay,
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
            replay_player: replay::Replay::new(),
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
        if self.player.skill_guage >= 80 {
            self.mode.push(BestAiMode::ModeBommer);
            return None;
        }

        if self.enemy.skill_guage >= 50 {
            self.mode.push(BestAiMode::ModeBommerKiller);
            return None;
        }
        
        if !self.replay_player.can_replay(&self.player) {
            let max_turn = if self.cur_turn <= 10 { 15 } else { 10 };
            let mut think_time_in_sec = if self.cur_turn == 0 { 19 } else { 15 };
            let mut enemy_send_obstacles = vec![0; max_turn];
            // if max_turn > 12 {
            //     enemy_send_obstacles[12] = 0;
            // }

            // emergency
            if self.rest_time_in_milli < 30 * 1000 {
                think_time_in_sec = 5;
            }

            let context = rensa_plan::PlanContext {
                plan_start_turn: self.cur_turn,
                max_turn: max_turn,
                think_time_in_sec,
                player: self.player.clone(),
                enemy_send_obstacles,
                packs: self.packs.clone(),
                stop_search_if_3_chains: true,
            };

            let states = rensa_plan::calc_rensa_plan(&context, |result, player, search_turn| {
                let obstacle_score = std::cmp::min(result.obstacle, 60);
                (obstacle_score * 100000 - search_turn as i32 * 1000 + result.obstacle * 16 + (self.rand.next() & 0xF) as i32)
            });

            let mut max = -1;
            let mut choosed = None;
            states.into_iter().for_each(|s| {
                if max < std::cmp::min(60, s.1.obstacle) {
                    max = std::cmp::min(60, s.1.obstacle);
                    choosed = Some(s);
                }
            });

            match choosed {
                None => return Self::resign(),
                Some(s) => {
                    if s.0.actions.is_empty() {
                        return Self::resign();
                    }

                    self.replay_player.init(&self.packs[self.cur_turn..], &s.0.actions, &s.1);
                    eprintln!("rensa: {} {} {}", self.cur_turn, s.0.actions.len(), s.1.chains);
                },
            }
        }

        self.replay_player.replay()
    }

    fn kill_bommer(&mut self) -> Option<action::Action> {
        if self.player.skill_guage >= 80 {
            self.mode.pop();
            return None;
        }

        if self.enemy.skill_guage <= 20 {
            self.mode.pop();
            return None;
        }

        if self.replay_player.is_empty() {
            let max_turn = 5;
            let think_time_in_sec = 3;
            let enemy_send_obstacles = vec![0; max_turn];

            let context = rensa_plan::PlanContext {
                plan_start_turn: self.cur_turn,
                max_turn: max_turn,
                think_time_in_sec,
                player: self.player.clone(),
                enemy_send_obstacles,
                packs: self.packs.clone(),
                stop_search_if_3_chains: false,
            };

            let states = rensa_plan::calc_rensa_plan(&context, |result, player, search_turn| {
                // result.skill_guage * 10000 + result.chains as i32 * 10 - search_turn as i32
                player.decrease_skill_guage * 10000 + result.chains as i32 * 10
            });

            let best = states.into_iter().max_by_key(|s| s.0.player.decrease_skill_guage).unwrap();
            self.replay_player.init(&self.packs[self.cur_turn..], &best.0.actions, &best.1);
            eprintln!("kill_bommer: {} {} {}", self.cur_turn, best.0.actions.len(), best.0.player.decrease_skill_guage);
        }

        self.replay_player.replay()
    }

    fn bommer(&mut self) -> Option<action::Action> {
        if self.player.skill_guage < 80 {
            self.mode.pop();
            return None;
        }

        let max_turn = 3;
        let context = skill_plan::PlanContext {
            plan_start_turn: self.cur_turn,
            max_turn: max_turn,
            think_time_in_sec: 1,
            player: self.player.clone(),
            enemy_send_obstacles: vec![0; max_turn],
        };
        let mut skill_plan = skill_plan::SkillPlan::new();

        skill_plan.set_pack(self.packs.clone());
        skill_plan.calc_skill_plan(&context);
        let replay = if self.player.can_use_skill() { action::Action::UseSkill } else { skill_plan.replay() };

        Some(replay)
    }

    fn resign() -> Option<action::Action> {
        Some(action::Action::PutBlock { pos: 0, rot: 0, })
    }
}
