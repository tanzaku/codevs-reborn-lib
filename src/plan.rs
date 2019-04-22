
use super::action;
use super::board;
use super::player;
use super::rand;

pub struct  {
}

pub struct Plans {
    best_plans: Vec<>,
}

impl Plan {
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
    
    fn calc_score(&mut self, result: &action::ActionResult, b: &board::Board, search_turn: usize) -> i32 {
        // let h = b.max_height() as i32;
        // -(result.obstacle * 10000 - h * 10 + search_turn as i32 * 16 + (self.rand.next() & 0xF) as i32)
        (result.obstacle * 10000 + search_turn as i32 * 16 + (self.rand.next() & 0xF) as i32)
    }
    
    // pub fn calc_rensa_plan(&mut self, cur_turn: usize, max_fire_turn: usize, player: &player::Player, ) {
    pub fn calc_rensa_plan(&mut self, context: &PlanContext) {
        let timer = Instant::now();

        // let max_fire_turn = if cur_turn == 0 { 13 } else { 10 };
        let actions = action::Action::all_actions();
        // let allow_dead_line = Self::is_dangerous(&player.board);

        let mut heaps = vec![BinaryHeap::new(); context.max_turn];

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
                        if &action::Action::UseSkill == a && !b.player.can_use_skill() {
                            return;
                        }

                        let mut player = b.player.clone();
                        let result = player.put(&pack, a);
                        if player.board.is_dead() {
                            return;
                        }

                        let mut actions = b.actions.clone();
                        actions.push(a.into());

                        let score = self.calc_score(&result, &player.board, search_turn);
                        if best.score < score {
                            best = BeamState::new(player.clone(), score, actions.clone());
                        }

                        if result.chains >= 3 {
                            return;
                        }

                        if cur_turn == 0 && player.board.max_height() >= H - 3 {
                            return;
                        }

                        if search_turn + 1 < max_fire_turn {
                            let max_score = (0..W).map(|x| (1..=9).map(|v| {
                                let mut rensa_eval_board = player.clone();
                                // let result = rensa_eval_board.put(&fall, &fire_action);
                                let result = rensa_eval_board.put_one(v, x as usize);
                                self.calc_score(&result, &player.board, search_turn)
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

    pub fn replay(&mut self) -> action::Action {
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


