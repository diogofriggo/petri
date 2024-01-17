use crate::error::Result;
use crate::model::{ActiveEvent, FeedingNode, Net, PassiveEvent, Transition};
use crate::tcp;
use glob::glob;
use std::collections::HashMap;
use std::hash::Hash;

pub struct Engine {
    pub clock: usize,
    pub step: usize,
    net: Net,
    last_clock: usize,
    fed_nodes: Vec<String>,
    feeding_nodes: Vec<FeedingNode>,
    transition2node: HashMap<usize, String>,
    events: Vec<ActiveEvent>,
}

impl Engine {
    pub fn new(last_clock: usize, node: String, nodes: &[&str], nets_folder: &str) -> Result<Self> {
        let mut nodes = nodes.to_vec();
        nodes.sort();
        nodes.dedup();

        let pattern = format!("{nets_folder}/*.json");
        let mut paths = glob(&pattern)?
            .filter_map(std::result::Result::ok)
            // .map(|path| path.display().to_string())
            .collect::<Vec<_>>();
        paths.sort();
        paths.dedup();

        let nets = paths
            .iter()
            .map(Net::new)
            .filter_map(std::result::Result::ok)
            .collect::<Vec<_>>();

        assert!(!nets.is_empty(), "No nets found at {}", nets_folder);
        assert!(!nodes.is_empty(), "No nodes provided");
        assert!(
            nets.len() == nodes.len(),
            "Number of nets differs from number of nodes"
        );

        let index = nodes.iter().position(|n| n == &node).unwrap();
        let net = nets[index].clone();

        let transition2node = nets
            .iter()
            .zip(nodes.iter())
            .flat_map(|(net, &node)| {
                net.transitions
                    .iter()
                    .map(|transition| (transition.id, node.into()))
            })
            .collect::<HashMap<usize, String>>();

        let node2fed_nodes: HashMap<String, Vec<String>> =
            nets.iter().fold(HashMap::new(), |mut acc, net| {
                net.transitions.iter().for_each(|transition| {
                    let node = transition2node[&transition.id].clone();
                    transition
                        .delayed_instructions
                        .iter()
                        .filter(|instruction| instruction.is_external)
                        .for_each(|instruction| {
                            let fed_node = transition2node[&instruction.transition_id].clone();
                            acc.entry(node.clone()).or_default().push(fed_node);
                        });
                });
                acc
            });

        let node2feeding_nodes = reverse(&node2fed_nodes);
        let feeding_nodes = node2feeding_nodes[&node]
            .iter()
            .map(|feeding_node| FeedingNode {
                name: feeding_node.clone(),
                clock: 0,
            })
            .collect::<Vec<_>>();

        let engine = Self {
            clock: 0,
            step: 1,
            net,
            last_clock,
            fed_nodes: node2fed_nodes[&node].clone(),
            feeding_nodes,
            transition2node,
            events: vec![],
        };

        Ok(engine)
    }

    pub fn run(&mut self) -> Result<()> {
        while self.clock < self.last_clock {
            let clock = self.clock;

            self.net
                .transitions
                .clone()
                .iter()
                .filter(|transition| transition.clock == clock && transition.value <= 0)
                .rev() // to simulate a stack
                .for_each(|transition| {
                    self.process_immediate_instructions(transition);
                    self.process_internal_delayed_instructions(transition);
                    self.process_external_delayed_instructions(transition);
                });

            self.tick();
            self.handle_events();
        }

        Ok(())
    }

    fn process_immediate_instructions(&mut self, transition: &Transition) {
        transition
            .immediate_instructions
            .iter()
            .for_each(|instruction| {
                self.net.transitions[instruction.transition_id].value = instruction.value;
            });
    }

    fn process_internal_delayed_instructions(&mut self, transition: &Transition) {
        let mut events = transition
            .delayed_instructions
            .iter()
            .filter(|instruction| !instruction.is_external)
            .map(|instruction| ActiveEvent {
                transition_id: instruction.transition_id,
                value: instruction.value,
                clock: transition.clock + transition.duration,
            })
            .collect::<Vec<_>>();

        self.events.append(&mut events);
    }

    fn process_external_delayed_instructions(&self, transition: &Transition) {
        self.fed_nodes
            .iter()
            .map(|fed_node| {
                let fed_node = fed_node.clone();
                let maybe_instruction = transition
                    .delayed_instructions
                    .iter()
                    .filter(|instruction| instruction.is_external)
                    .find(|instruction| {
                        let destination_node =
                            self.transition2node[&instruction.transition_id].clone();
                        destination_node == fed_node
                    });

                let event: &[u8] = match maybe_instruction {
                    Some(instruction) => ActiveEvent {
                        transition_id: instruction.transition_id,
                        value: instruction.value,
                        clock: transition.clock + transition.duration,
                    }
                    .into(),
                    None => PassiveEvent {
                        clock: self.clock + self.step,
                    }
                    .into(),
                };

                (fed_node.clone(), event)
            })
            .for_each(|(destination_node, event): (String, &[u8])| {
                tcp::send(&destination_node, event);
            });
    }

    fn tick(&mut self) {
        let earliest_clock = self
            .events
            .iter()
            .map(|event| event.clock)
            .chain(
                self.feeding_nodes
                    .iter()
                    .map(|feeding_node| feeding_node.clock),
            )
            .min()
            .unwrap_or(self.clock);

        for feeding_node in self.feeding_nodes.iter_mut() {
            if feeding_node.clock == earliest_clock {
                // I cannot block on a single channel, rather block on all of them together
                // for message in feeding_node.channel {
                //
                // }
                // wait on channel, that is, block until we have either an active or passive message
            }
            // then here he goes on to read any remaining event without blocking
        }

        self.clock = self
            .events
            .iter()
            .map(|event| event.clock)
            .min()
            .unwrap_or(self.clock + self.step);
    }

    // func (se *SimulationEngine) forwardTime() Clock {
    // 	var lowerBoundClock Clock
    //
    // 	// Initial min time is first event clock
    // 	if lowerBoundClock = se.eventList.firstEventClock(); lowerBoundClock == -1 {
    // 		lowerBoundClock = se.clock
    // 	}
    //
    // 	// If any waiting segment has a lower clock set as minTime
    // 	for _, v := range se.waitingOnSegments {
    // 		if v.clock < lowerBoundClock {
    // 			lowerBoundClock = v.clock
    // 		}
    // 	}
    //
    // 	// Wait for the lowest clock segments wither by event, or by lookahead
    // 	for _, v := range se.waitingOnSegments {
    // 		if v.clock == lowerBoundClock {
    // 			select {
    // 			case clock := <-v.lookahead:
    // 				v.clock = clock
    // 			case event := <-v.eventQueue:
    // 				se.eventList.insert(event)
    // 			}
    // 		}
    // 	Loop:
    // 		for {
    // 			select {
    // 			case event := <-v.eventQueue:
    // 				se.eventList.insert(event)
    // 			default:
    // 				break Loop
    // 			}
    // 		}
    // 	}
    //
    // 	if lowerBoundClock = se.eventList.firstEventClock(); lowerBoundClock == -1 {
    // 		lowerBoundClock = se.clock + se.lookahead
    // 	}
    //
    // 	return lowerBoundClock
    // }

    fn handle_events(&mut self) {
        // below events are ordered from lowest clock to highest clock,
        // but if we always handle events for the current clock then there's no need to do any sorting
        // self.events.sort_by(|a, b| a.clock.cmp(&b.clock));

        self.events
            .iter()
            .filter(|event| event.clock == self.clock)
            .for_each(|event| {
                if let Some(transition) = &mut self
                    .net
                    .transitions
                    .iter_mut()
                    .find(|transition| transition.id == event.transition_id)
                {
                    transition.clock = event.clock;
                    transition.value = event.value;
                }
            });

        self.events.retain(|event| event.clock != self.clock);
    }
}

fn reverse<K, V>(input: &HashMap<K, Vec<V>>) -> HashMap<V, Vec<K>>
where
    K: Eq + Hash + Clone,
    V: Eq + Hash + Clone,
{
    let mut output: HashMap<V, Vec<K>> = HashMap::new();

    for (key, values) in input {
        for value in values {
            output.entry(value.clone()).or_default().push(key.clone());
        }
    }

    output
}
