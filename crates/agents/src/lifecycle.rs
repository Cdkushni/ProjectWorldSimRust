use parking_lot::RwLock;
use rand::Rng;
use std::sync::Arc;
use world_sim_core::{AgentId, Position};
use world_sim_event_bus::{AgentBornEvent, AgentDiedEvent, EventBus};
use crate::{AgentState, SimAgent};

/// Manages the birth, death, and population of agents
pub struct LifecycleLayer {
    agents: Arc<RwLock<Vec<SimAgent>>>,
    birth_rate: f32,
    death_rate: f32,
    event_bus: Arc<EventBus>,
}

impl LifecycleLayer {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            agents: Arc::new(RwLock::new(Vec::new())),
            birth_rate: 0.001,
            death_rate: 0.001,
            event_bus,
        }
    }

    /// Add a new agent to the simulation
    pub fn spawn_agent(&self, agent: SimAgent) {
        let mut agents = self.agents.write();
        agents.push(agent);
    }

    /// Create a new agent (birth or immigration)
    pub async fn birth_agent(&self, name: String, position: Position, parents: Vec<AgentId>) {
        let agent = SimAgent::new(name, position);
        let id = agent.id;
        
        self.spawn_agent(agent);
        
        // Publish birth event
        self.event_bus
            .publish(&AgentBornEvent {
                agent_id: id,
                parent_ids: parents,
                location: position,
            })
            .await;
    }

    /// Kill an agent
    pub async fn kill_agent(&self, agent_id: AgentId, cause: String) {
        let mut agents = self.agents.write();
        
        if let Some(agent) = agents.iter_mut().find(|a| a.id == agent_id) {
            let position = agent.position;
            agent.state = AgentState::Dead;
            
            // Publish death event
            self.event_bus
                .publish(&AgentDiedEvent {
                    agent_id,
                    cause,
                    location: position,
                })
                .await;
        }
    }

    /// Process natural births and deaths
    pub async fn tick(&self) {
        let mut rng = rand::thread_rng();
        let agents = self.agents.read();
        let agent_count = agents.len();
        drop(agents);
        
        // Random births
        if rng.gen::<f32>() < self.birth_rate * agent_count as f32 {
            let position = Position::new(
                rng.gen_range(-100.0..100.0),
                1.0,
                rng.gen_range(-100.0..100.0),
            );
            self.birth_agent(format!("Citizen_{}", rng.gen::<u32>()), position, vec![])
                .await;
        }
        
        // Random deaths (natural causes)
        let agents = self.agents.read();
        let alive_agents: Vec<AgentId> = agents
            .iter()
            .filter(|a| a.is_alive())
            .map(|a| a.id)
            .collect();
        drop(agents);
        
        for agent_id in alive_agents {
            if rng.gen::<f32>() < self.death_rate {
                self.kill_agent(agent_id, "Natural causes".to_string())
                    .await;
            }
        }
    }

    /// Get all agents
    pub fn get_agents(&self) -> Vec<SimAgent> {
        self.agents.read().clone()
    }

    /// Get agent by ID
    pub fn get_agent(&self, id: AgentId) -> Option<SimAgent> {
        self.agents.read().iter().find(|a| a.id == id).cloned()
    }

    /// Count living agents
    pub fn count_living(&self) -> usize {
        self.agents.read().iter().filter(|a| a.is_alive()).count()
    }
}

