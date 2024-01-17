use crate::{engine::Engine, error::Result};
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

#[derive(Debug, Clone)]
pub struct ActiveEvent {
    pub transition_id: usize,
    pub value: isize,
    pub clock: usize,
}

#[derive(Debug, Clone)]
pub struct PassiveEvent {
    pub clock: usize,
}

impl From<ActiveEvent> for &[u8] {
    fn from(value: ActiveEvent) -> Self {
        todo!()
    }
}

impl From<PassiveEvent> for &[u8] {
    fn from(value: PassiveEvent) -> Self {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct FeedingNode {
    pub name: String,
    pub clock: usize,
}
