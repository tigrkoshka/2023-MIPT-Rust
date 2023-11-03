#![forbid(unsafe_code)]

use crate::Decision::{Cheat, Cooperate};
use crate::RoundOutcome::{BothCheated, BothCooperated, LeftCheated, RightCheated};

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundOutcome {
    BothCooperated,
    LeftCheated,
    RightCheated,
    BothCheated,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Cheat,
    Cooperate,
}

pub trait Agent {
    fn play(&mut self) -> Decision;
    fn update(&mut self, _decision: Decision) {}
}

pub struct Game {
    left: Box<dyn Agent>,
    right: Box<dyn Agent>,
    left_score: i32,
    right_score: i32,
}

impl Game {
    pub fn new(left: Box<dyn Agent>, right: Box<dyn Agent>) -> Self {
        Game {
            left,
            right,
            left_score: 0,
            right_score: 0,
        }
    }

    pub fn left_score(&self) -> i32 {
        self.left_score
    }

    pub fn right_score(&self) -> i32 {
        self.right_score
    }

    pub fn play_round(&mut self) -> RoundOutcome {
        let left_decision = self.left.play();
        let right_decision = self.right.play();

        self.left.update(right_decision);
        self.right.update(left_decision);

        match left_decision {
            Cheat => match right_decision {
                Cheat => BothCheated,
                Cooperate => {
                    self.left_score += 3;
                    self.right_score -= 1;
                    LeftCheated
                }
            },

            Cooperate => match right_decision {
                Cheat => {
                    self.right_score += 3;
                    self.left_score -= 1;
                    RightCheated
                }
                Cooperate => {
                    self.left_score += 2;
                    self.right_score += 2;
                    BothCooperated
                }
            },
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct CheatingAgent {}

impl CheatingAgent {
    pub fn new() -> Self {
        CheatingAgent {}
    }
}

impl Agent for CheatingAgent {
    fn play(&mut self) -> Decision {
        Cheat
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct CooperatingAgent {}

impl CooperatingAgent {
    pub fn new() -> Self {
        CooperatingAgent {}
    }
}

impl Agent for CooperatingAgent {
    fn play(&mut self) -> Decision {
        Cooperate
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct GrudgerAgent {
    deceived: bool,
}

impl GrudgerAgent {
    pub fn new() -> Self {
        GrudgerAgent { deceived: false }
    }
}

impl Agent for GrudgerAgent {
    fn play(&mut self) -> Decision {
        if self.deceived {
            Cheat
        } else {
            Cooperate
        }
    }

    fn update(&mut self, decision: Decision) {
        if decision == Cheat {
            self.deceived = true
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct CopycatAgent {
    previous: Option<Decision>,
}

impl CopycatAgent {
    pub fn new() -> Self {
        CopycatAgent { previous: None }
    }
}

impl Agent for CopycatAgent {
    fn play(&mut self) -> Decision {
        match self.previous {
            Some(x) => x,
            None => Cooperate,
        }
    }

    fn update(&mut self, decision: Decision) {
        self.previous = Some(decision);
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct DetectiveAgent {
    i: i32,
    deceived: bool,
    previous: Decision,
}

impl Default for DetectiveAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl DetectiveAgent {
    pub fn new() -> Self {
        DetectiveAgent {
            i: 0,
            deceived: false,
            previous: Cooperate,
        }
    }
}

impl Agent for DetectiveAgent {
    fn play(&mut self) -> Decision {
        self.i += 1;
        if self.i == 2 {
            Cheat
        } else if self.i <= 4 {
            Cooperate
        } else if !self.deceived {
            Cheat
        } else {
            self.previous
        }
    }

    fn update(&mut self, decision: Decision) {
        self.previous = decision;
        if decision == Cheat {
            self.deceived = true
        }
    }
}
