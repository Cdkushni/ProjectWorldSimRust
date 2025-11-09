use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use world_sim_core::{AgentId, BlockType, GridCoord, ItemId, Position};

/// Represents something perceptible in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Stimulus {
    Visual {
        source: Position,
        stimulus_type: VisualStimulus,
    },
    Auditory {
        source: Position,
        stimulus_type: AuditoryStimulus,
        loudness: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VisualStimulus {
    Agent(AgentId),
    Item(ItemId),
    Block(GridCoord, BlockType),
    Action(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditoryStimulus {
    Speech(AgentId, String),
    Combat,
    Explosion,
    AnimalNoise,
}

/// Global stimulus broadcaster
pub struct StimulusSubsystem {
    stimuli: Arc<RwLock<Vec<Stimulus>>>,
}

impl StimulusSubsystem {
    pub fn new() -> Self {
        Self {
            stimuli: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Broadcast a stimulus
    pub fn broadcast(&self, stimulus: Stimulus) {
        self.stimuli.write().push(stimulus);
    }

    /// Get all current stimuli and clear the buffer
    pub fn collect_and_clear(&self) -> Vec<Stimulus> {
        let mut stimuli = self.stimuli.write();
        let collected = stimuli.clone();
        stimuli.clear();
        collected
    }
}

impl Default for StimulusSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

/// An agent's perception component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPerception {
    pub sight_radius: f32,
    pub hearing_radius: f32,
    pub sight_cone_angle: f32, // In degrees
    pub known_world: KnownWorld,
}

/// The agent's subjective view of the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownWorld {
    pub known_agents: HashMap<AgentId, KnownAgent>,
    pub known_items: HashMap<ItemId, KnownItem>,
    pub known_blocks: HashMap<GridCoord, BlockType>,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownAgent {
    pub id: AgentId,
    pub last_seen_position: Position,
    pub last_seen_time: u64,
    pub known_traits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownItem {
    pub id: ItemId,
    pub item_type: String,
    pub last_seen_position: Position,
    pub last_seen_time: u64,
}

impl AgentPerception {
    pub fn new() -> Self {
        Self {
            sight_radius: 50.0,
            hearing_radius: 100.0,
            sight_cone_angle: 120.0,
            known_world: KnownWorld {
                known_agents: HashMap::new(),
                known_items: HashMap::new(),
                known_blocks: HashMap::new(),
                last_updated: 0,
            },
        }
    }

    /// Process stimuli and update known world
    pub fn process_stimuli(
        &mut self,
        agent_position: Position,
        stimuli: &[Stimulus],
        current_time: u64,
    ) {
        for stimulus in stimuli {
            match stimulus {
                Stimulus::Visual { source, stimulus_type } => {
                    let distance = agent_position.distance_to(source);
                    if distance <= self.sight_radius {
                        self.process_visual_stimulus(stimulus_type, *source, current_time);
                    }
                }
                Stimulus::Auditory { source, stimulus_type, loudness } => {
                    let distance = agent_position.distance_to(source);
                    let effective_range = self.hearing_radius * loudness;
                    if distance <= effective_range {
                        self.process_auditory_stimulus(stimulus_type, *source, current_time);
                    }
                }
            }
        }
        
        self.known_world.last_updated = current_time;
    }

    fn process_visual_stimulus(&mut self, stimulus: &VisualStimulus, source: Position, time: u64) {
        match stimulus {
            VisualStimulus::Agent(agent_id) => {
                self.known_world.known_agents.insert(
                    *agent_id,
                    KnownAgent {
                        id: *agent_id,
                        last_seen_position: source,
                        last_seen_time: time,
                        known_traits: Vec::new(),
                    },
                );
            }
            VisualStimulus::Item(item_id) => {
                self.known_world.known_items.insert(
                    *item_id,
                    KnownItem {
                        id: *item_id,
                        item_type: "unknown".to_string(),
                        last_seen_position: source,
                        last_seen_time: time,
                    },
                );
            }
            VisualStimulus::Block(coord, block_type) => {
                self.known_world.known_blocks.insert(*coord, *block_type);
            }
            _ => {}
        }
    }

    fn process_auditory_stimulus(&mut self, _stimulus: &AuditoryStimulus, _source: Position, _time: u64) {
        // Process audio cues (implementation depends on game design)
    }

    /// Check if agent knows about another agent
    pub fn knows_agent(&self, agent_id: AgentId) -> bool {
        self.known_world.known_agents.contains_key(&agent_id)
    }

    /// Get known position of an agent (if any)
    pub fn get_known_agent_position(&self, agent_id: AgentId) -> Option<Position> {
        self.known_world
            .known_agents
            .get(&agent_id)
            .map(|ka| ka.last_seen_position)
    }
}

impl Default for AgentPerception {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perception() {
        let mut perception = AgentPerception::new();
        let agent_pos = Position::new(0.0, 0.0, 0.0);
        let other_agent = AgentId::new();
        
        let stimulus = Stimulus::Visual {
            source: Position::new(10.0, 0.0, 0.0),
            stimulus_type: VisualStimulus::Agent(other_agent),
        };
        
        perception.process_stimuli(agent_pos, &[stimulus], 0);
        assert!(perception.knows_agent(other_agent));
    }
}

