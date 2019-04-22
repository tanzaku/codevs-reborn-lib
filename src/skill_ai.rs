

use std::str::FromStr;
use std::io::Read;
use std::io::StdinLock;

use super::action;
use super::board;
use super::player;
use super::skill_plan;
use super::consts::{W,H,MAX_TURN};

pub struct SkillAi<'a> {
    cur_turn: usize,
    stdin_lock: StdinLock<'a>,
    packs: Vec<[[u8; 2]; 2]>,
    prev_obstacle_stock: i32,
    player: player::Player,
    enemy: player::Player,
    skill_plan: skill_plan::SkillPlan,
}

impl<'a> SkillAi<'a> {
    pub fn new(lock: StdinLock<'a>) -> Self {
        Self {
            cur_turn: 0,
            stdin_lock: lock,
            packs: Vec::new(),
            prev_obstacle_stock: 0,
            player: player::Player::new(board::Board::new(), 0, 0),
            enemy: player::Player::new(board::Board::new(), 0, 0),
            skill_plan: skill_plan::SkillPlan::new(),
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
        self.skill_plan.set_pack(self.packs.clone());
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
        let _rest_time_in_milli = self.read1::<u32>();
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
        println!("test-skill-ai");
        self.read_game_input();
        loop {
            self.read_turn_input();
            let act = self.think();
            println!("{}", act);
        }
    }

    fn think(&mut self) -> action::Action {
        let max_turn = 3;
        let context = skill_plan::PlanContext {
            plan_start_turn: self.cur_turn,
            max_turn: max_turn,
            think_time_in_sec: 1,
            player: self.player.clone(),
            enemy_send_obstacles: vec![0; max_turn],
        };
        self.skill_plan.calc_skill_plan(&context);
        self.skill_plan.replay()
    }
}
