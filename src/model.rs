use serde::{Deserialize, Serialize};

use crate::error::Result;
use std::fmt::Display;
use std::sync::mpsc::Receiver;
use std::{fs::File, io::BufReader, path::Path};

#[derive(Debug, Clone)]
pub struct Net {
    pub transitions: Vec<Transition>,
}

impl Net {
    pub fn new<T: AsRef<Path>>(path: T) -> Result<Net> {
        let file = File::open(path)?;
        let file = BufReader::new(file);
        let net: crate::json::Net = serde_json::from_reader(file)?;

        let transitions = net
            .ia_red
            .into_iter()
            .map(|transition| Transition {
                id: transition.ii_idglobal,
                value: transition.ii_valor,
                clock: transition.ii_tiempo,
                duration: transition.ii_duracion_disparo,
                immediate_instructions: parse_instructions(&transition.ii_listactes_iul),
                delayed_instructions: parse_instructions(&transition.ii_listactes_pul),
                is_output: transition.ib_desalida,
            })
            .collect();

        let net = Self { transitions };

        Ok(net)
    }
}

fn parse_instructions(instructions: &[(isize, isize)]) -> Vec<Instruction> {
    instructions.iter().map(Instruction::new).collect()
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub id: usize,
    pub value: isize,
    pub clock: usize,
    pub duration: usize,
    pub immediate_instructions: Vec<Instruction>,
    pub delayed_instructions: Vec<Instruction>,
    pub is_output: bool,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub transition_id: usize,
    pub value: isize,
    pub is_external: bool,
}

impl Instruction {
    pub fn new(instruction: &(isize, isize)) -> Self {
        let transition_id = instruction.0;
        let is_external = transition_id < 0;
        let transition_id = if is_external {
            -(transition_id + 1)
        } else {
            transition_id
        } as usize;

        Self {
            transition_id,
            value: instruction.1,
            is_external,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveEvent {
    pub feeding_node: String,
    pub transition_id: usize,
    pub value: isize,
    pub clock: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassiveEvent {
    pub feeding_node: String,
    pub clock: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericEvent {
    pub feeding_node: String,
}

impl From<ActiveEvent> for String {
    fn from(value: ActiveEvent) -> Self {
        serde_json::to_string(&value).unwrap()
    }
}

impl From<PassiveEvent> for String {
    fn from(value: PassiveEvent) -> Self {
        serde_json::to_string(&value).unwrap()
    }
}

#[derive(Debug)]
pub struct FeedingNode {
    pub name: String,
    pub clock: usize,
    pub channel: Receiver<String>,
}

impl Display for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "id={} clock={} value={}",
            self.id, self.clock, self.value
        )
    }
}

impl Display for Net {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let transitions = self
            .transitions
            .iter()
            .map(|transition| format!("{transition}"))
            .collect::<Vec<_>>();

        write!(f, "{}", transitions.join(" |___| "))
    }
}

impl Display for FeedingNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.name, self.clock)
    }
}
