

use std::str::FromStr;
use std::io::Read;
use std::io::StdinLock;

use super::action;
use super::board;
use super::player;
use super::rensa_plan;
use super::skill_plan;
use super::consts::{W,H,MAX_TURN};

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
    rensa_plan: rensa_plan::RensaPlan,
    mode: BestAiMode,
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
            rensa_plan: rensa_plan::RensaPlan::new(),
            mode: BestAiMode::ModeRensa,
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
        self.rensa_plan.set_pack(self.packs.clone());
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
        // loop {
        //     if self.mode == BestAiMode::ModeRensa {
        //         if self.player.skill_guage >= 80 {
        //             self.mode = BestAiMode::ModeBommer;
        //         }

        //         if self.enemy.skill_guage >= 50 {
        //             self.mode = BestAiMode::ModeBommerKiller;
        //         }
        //     }

        //     if let Some(a) = match self.mode {
        //         BestAiMode::ModeBommerKiller => self.rensa_plan.replay(),
        //         BestAiMode::ModeRecalcRensa => self.rensa_plan.replay(),
        //         BestAiMode::ModeRensa => self.rensa_plan.replay(),
        //         BestAiMode::ModeSkill => None,
        //     }
        // }


        if self.mode == BestAiMode::ModeBommerKiller && self.enemy.skill_guage < 20 {
            self.mode = BestAiMode::ModeRensa;
            self.rensa_plan.clear_replay();
        }

        if self.mode == BestAiMode::ModeRensa && (!self.rensa_plan.can_replay(&self.player) || self.new_obstscle()) {
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
            };

            self.rensa_plan.calc_rensa_plan(&context);
        }

        if self.mode != BestAiMode::ModeBommerKiller {
            if self.player.skill_guage >= 80 {
                self.mode = BestAiMode::ModeBommer;
            }
        }

        // if self.mode != BestAiMode::ModeBommerKiller && self.enemy.skill_guage >= 60 || self.mode == BestAiMode::ModeBommerKiller && !self.rensa_plan.exists() {
        //     self.mode = BestAiMode::ModeBommerKiller;
        //     self.rensa_plan.clear_replay();

        //     let max_turn = 2;
        //     let think_time_in_sec = 1;
        //     let enemy_send_obstacles = vec![0; max_turn];

        //     let context = rensa_plan::PlanContext {
        //         plan_start_turn: self.cur_turn,
        //         max_turn: max_turn,
        //         think_time_in_sec,
        //         player: self.player.clone(),
        //         enemy_send_obstacles,
        //     };

        //     self.rensa_plan.calc_rensa_plan(&context);
        // }

        match self.mode {
            BestAiMode::ModeBommerKiller => self.rensa_plan.replay(),
            BestAiMode::ModeRensa => self.rensa_plan.replay(),
            BestAiMode::ModeBommer => {
                self.rensa_plan.clear_replay();

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
                let replay = skill_plan.replay();
                let replay = action::Action::UseSkill;

                if replay == action::Action::UseSkill {
                    self.mode = BestAiMode::ModeRensa;
                }

                replay
            },
        }
    }

    fn new_obstscle(&self) -> bool {
        let w = W as i32;
        self.player.obstacle >= w && (self.prev_obstacle_stock - w) / w != self.player.obstacle / w
    }
}
