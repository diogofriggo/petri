use crate::error::Result;
use crate::model::{ActiveEvent, FeedingNode, GenericEvent, Net, PassiveEvent, Transition};
use chrono::Local;
use glob::glob;
use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::mpsc::channel;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct Engine {
    clock: usize,
    step: usize,
    node: String,
    net: Net,
    terminal_clock: usize,
    fed_nodes: Vec<String>,
    feeding_nodes: Vec<FeedingNode>,
    transition2node: HashMap<usize, String>,
    internal_active_events: Vec<ActiveEvent>,
    external_active_events: Vec<ActiveEvent>,
    pub listener: JoinHandle<Result<()>>,
    log_file: BufWriter<File>,
}

impl Engine {
    pub fn new(
        terminal_clock: usize,
        node: String,
        nodes: &[String],
        nets_folder: &Path,
    ) -> Result<Self> {
        let log_path = format!("{}.log", node);
        let log_file = File::create(log_path)?;
        let log_file = BufWriter::new(log_file);

        let mut nodes = nodes.to_vec();
        nodes.sort();
        nodes.dedup();

        let nets_folder = nets_folder.display();
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
            .flat_map(|(net, node)| {
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
        let fed_nodes = node2fed_nodes[&node].clone();

        let node2feeding_nodes = reverse_hashmap(&node2fed_nodes);
        let (feeding_node2channel, feeding_nodes): (HashMap<_, _>, Vec<_>) = node2feeding_nodes
            [&node]
            .iter()
            .map(|feeding_node| {
                let (tx, rx) = channel();
                let feeding_node = FeedingNode {
                    name: feeding_node.clone(),
                    clock: 0,
                    channel: rx,
                };
                ((feeding_node.name.clone(), tx), feeding_node)
            })
            .unzip();

        let node_clone = node.clone();
        let listener = thread::spawn(move || -> Result<()> {
            let msg = format!("Failed to listen on {}", node_clone);
            for stream in TcpListener::bind(node_clone.clone())
                .expect(&msg)
                .incoming()
            {
                let mut reader = BufReader::new(stream?);
                let mut event: String = Default::default();
                reader.read_line(&mut event)?;

                if let Ok(GenericEvent { feeding_node }) = serde_json::from_str(&event) {
                    // avoided generic error
                    let msg = format!("Failed to channel event to {}", feeding_node);
                    feeding_node2channel[&feeding_node].send(event).expect(&msg);
                } else {
                    unreachable!("GenericEvent could not be parsed");
                }
            }

            Ok(())
        });

        let engine = Self {
            clock: 0,
            step: 1,
            node,
            net,
            terminal_clock,
            fed_nodes,
            feeding_nodes,
            transition2node,
            internal_active_events: vec![],
            external_active_events: vec![],
            listener,
            log_file,
        };

        Ok(engine)
    }

    pub fn run(&mut self) -> Result<()> {
        while self.clock < self.terminal_clock {
            self.log(&format!("LOOP START            {}", self.net));
            let clock = self.clock;

            self.net
                .transitions
                .clone()
                .iter()
                .filter(|transition| transition.clock == clock && transition.value <= 0)
                .rev() // to simulate a stack
                .for_each(|transition| {
                    self.process_immediate_instructions(transition);
                    self.process_delayed_instructions(transition);
                });
            self.log(&format!("AFTER INSTRUCTIONS    {}", self.net));

            self.handle_external_events()?;
            self.external_active_events.clear();
            self.log(&format!("AFTER EXTERNAL EVENTS {}", self.net));

            self.tick()?;
            self.log(&format!("AFTER TICK            {}", self.net));

            self.handle_internal_events();
            self.log(&format!("AFTER INTERNAL EVENTS {}", self.net));
        }

        self.log(&format!("FINISHED              {}", self.net));

        Ok(())
    }

    fn process_immediate_instructions(&mut self, transition: &Transition) {
        transition
            .immediate_instructions
            .iter()
            .for_each(|instruction| {
                if let Some(transition) = self
                    .net
                    .transitions
                    .iter_mut()
                    .find(|transition| transition.id == instruction.transition_id)
                {
                    transition.value = instruction.value;
                } else {
                    unreachable!("Instruction referenced a non-existing transition");
                }
            });
    }

    fn process_delayed_instructions(&mut self, transition: &Transition) {
        transition
            .delayed_instructions
            .iter()
            .for_each(|instruction| {
                let event = ActiveEvent {
                    transition_id: instruction.transition_id,
                    feeding_node: self.node.clone(),
                    value: instruction.value,
                    clock: transition.clock + transition.duration,
                };
                if instruction.is_external {
                    self.external_active_events.push(event);
                } else {
                    self.internal_active_events.push(event);
                }
            });
    }

    fn handle_external_events(&mut self) -> Result<()> {
        let active_events = self
            .external_active_events
            .clone()
            .into_iter()
            .map(|event| {
                let fed_node = &self.transition2node[&event.transition_id];
                (fed_node.clone(), event.into())
            })
            .collect::<Vec<(String, String)>>();

        let covered_nodes = active_events
            .iter()
            .map(|(node, _)| node)
            .collect::<Vec<_>>();

        let passive_events = self
            .fed_nodes
            .iter()
            .filter(|fed_node| !covered_nodes.contains(fed_node))
            .map(|fed_node| {
                let event = PassiveEvent {
                    feeding_node: self.node.clone(),
                    clock: self.clock + self.step,
                };
                (fed_node.clone(), event.into())
            })
            .collect::<Vec<(String, String)>>();

        active_events
            .into_iter()
            .chain(passive_events)
            .try_for_each(|(fed_node, event): (String, String)| -> Result<()> {
                // not sure I really need this new line, I do this bc the listening tcp stream
                // will consider \n as a message terminator
                let event = format!("{event}\n");
                let payload = event.as_bytes();
                // at the beginning of execution we need to wait until
                // all other nodes are ready to listen
                match TcpStream::connect(&fed_node) {
                    Ok(mut stream) => stream.write_all(payload)?,
                    Err(_) => {
                        thread::sleep(Duration::from_secs(3));
                        let mut stream = TcpStream::connect(&fed_node)?;
                        let msg = format!("Failed to write to {}", fed_node);
                        stream.write_all(payload).expect(&msg);
                        self.log(&format!("SENT {}", event));
                    }
                };

                Ok(())
            })
    }

    fn tick(&mut self) -> Result<()> {
        let earliest_clock = self
            .internal_active_events
            .iter()
            .map(|event| event.clock)
            .chain(
                self.feeding_nodes
                    .iter()
                    .map(|feeding_node| feeding_node.clock),
            )
            .min()
            .unwrap_or(self.clock);

        let events = self
            .feeding_nodes
            .iter()
            .filter(|feeding_node| feeding_node.clock == earliest_clock)
            .map(|feeding_node| feeding_node.channel.recv())
            .filter_map(std::result::Result::ok)
            .chain(
                // catches any extra events other than the above mandatory ones without blocking
                // otherwise feeding nodes that are not at `earliest_clock` would miss events
                self.feeding_nodes
                    .iter()
                    .map(|feeding_node| feeding_node.channel.try_recv())
                    .filter_map(std::result::Result::ok),
            )
            .collect::<Vec<_>>();

        events.into_iter().for_each(|event| {
            if let Ok(event @ ActiveEvent { .. }) = serde_json::from_str(&event) {
                self.log(&format!("RECEIVED {:?}", event));
                self.internal_active_events.push(event);
            } else if let Ok(event @ PassiveEvent { .. }) = serde_json::from_str(&event) {
                self.log(&format!("RECEIVED {:?}", event));
                if let Some(feeding_node) = self
                    .feeding_nodes
                    .iter_mut()
                    .find(|feeding_node| feeding_node.name == event.feeding_node)
                {
                    feeding_node.clock = event.clock;
                }
            } else {
                unreachable!("Event could not be parsed");
            }
        });

        self.clock = self
            .internal_active_events
            .iter()
            .map(|event| event.clock)
            .min()
            .unwrap_or(self.clock + self.step);

        Ok(())
    }

    fn handle_internal_events(&mut self) {
        // below events are ordered from lowest clock to highest clock,
        // but if we always handle events for the current clock then there's no need to do any sorting
        // self.events.sort_by(|a, b| a.clock.cmp(&b.clock));

        self.internal_active_events
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

        self.internal_active_events
            .retain(|event| event.clock != self.clock);
    }

    fn log(&mut self, msg: &str) {
        log(&mut self.log_file, self.clock, &self.node, msg);
    }
}

fn log(file: &mut BufWriter<File>, clock: usize, node: &str, msg: &str) {
    let stamp = Local::now().format("%Y-%m-%d %H:%M:%S.%f");
    let data = format!("[{}] [clk={}] [node={}] {}\n", stamp, clock, node, msg);
    file.write_all(data.as_bytes()).unwrap();
}

fn reverse_hashmap<K, V>(input: &HashMap<K, Vec<V>>) -> HashMap<V, Vec<K>>
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
