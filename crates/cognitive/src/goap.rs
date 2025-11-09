use ahash::AHashSet;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use world_sim_world::ActionDefinition;

use crate::Goal;

/// GOAP (Goal-Oriented Action Planning) - The tactical planner
pub struct GOAPPlanner {
    pub actions: Vec<ActionDefinition>,
}

/// A world state for planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub facts: HashSet<String>,
}

impl WorldState {
    pub fn new() -> Self {
        Self {
            facts: HashSet::new(),
        }
    }

    pub fn has(&self, fact: &str) -> bool {
        self.facts.contains(fact)
    }

    pub fn set(&mut self, fact: String) {
        self.facts.insert(fact);
    }

    pub fn remove(&mut self, fact: &str) {
        self.facts.remove(fact);
    }
}

impl Default for WorldState {
    fn default() -> Self {
        Self::new()
    }
}

/// A node in the GOAP search
#[derive(Clone)]
struct PlanNode {
    state: WorldState,
    actions: Vec<String>,
    cost: f32,
    heuristic: f32,
}

impl PlanNode {
    fn total_cost(&self) -> f32 {
        self.cost + self.heuristic
    }
}

impl Eq for PlanNode {}

impl PartialEq for PlanNode {
    fn eq(&self, other: &Self) -> bool {
        self.total_cost() == other.total_cost()
    }
}

impl Ord for PlanNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.total_cost().partial_cmp(&self.total_cost()).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for PlanNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl GOAPPlanner {
    pub fn new(actions: Vec<ActionDefinition>) -> Self {
        Self { actions }
    }

    /// Plan using regressive A* search
    pub fn plan(
        &self,
        current_state: &WorldState,
        goal: &Goal,
        max_iterations: usize,
    ) -> Option<Vec<String>> {
        // Create goal state
        let mut goal_state = WorldState::new();
        goal_state.set(goal.condition.clone());

        // Start from goal and work backwards
        let mut open_set = BinaryHeap::new();
        let mut closed_set = AHashSet::new();

        open_set.push(PlanNode {
            state: goal_state.clone(),
            actions: Vec::new(),
            cost: 0.0,
            heuristic: 0.0,
        });

        let mut iterations = 0;

        while let Some(current) = open_set.pop() {
            iterations += 1;
            if iterations > max_iterations {
                return None; // Timeout
            }

            // Check if we've reached the current state
            if self.states_match(current_state, &current.state) {
                return Some(current.actions);
            }

            // Generate state hash for closed set
            let state_hash = self.hash_state(&current.state);
            if closed_set.contains(&state_hash) {
                continue;
            }
            closed_set.insert(state_hash);

            // Try each action (regressive search)
            for action in &self.actions {
                if self.can_apply_regressive(action, &current.state) {
                    let mut new_state = current.state.clone();
                    self.apply_regressive(action, &mut new_state);

                    let mut new_actions = current.actions.clone();
                    new_actions.insert(0, action.id.clone());

                    let heuristic = self.calculate_heuristic(&new_state, current_state);

                    open_set.push(PlanNode {
                        state: new_state,
                        actions: new_actions,
                        cost: current.cost + action.base_cost,
                        heuristic,
                    });
                }
            }
        }

        None // No plan found
    }

    /// Check if an action can be applied in regressive planning
    fn can_apply_regressive(&self, action: &ActionDefinition, state: &WorldState) -> bool {
        // In regressive planning, we check if the effects are needed
        for effect in &action.effects {
            if state.has(effect) {
                return true;
            }
        }
        false
    }

    /// Apply action in regressive planning (backwards)
    fn apply_regressive(&self, action: &ActionDefinition, state: &mut WorldState) {
        // Remove effects (we're working backwards)
        for effect in &action.effects {
            state.remove(effect);
        }

        // Add preconditions (these are now the new goals)
        for precondition in &action.preconditions {
            state.set(precondition.clone());
        }
    }

    /// Check if two states match
    fn states_match(&self, state_a: &WorldState, state_b: &WorldState) -> bool {
        // Check if all facts in state_b are in state_a
        for fact in &state_b.facts {
            if !state_a.has(fact) {
                return false;
            }
        }
        true
    }

    /// Hash a state for closed set
    fn hash_state(&self, state: &WorldState) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        let mut facts: Vec<_> = state.facts.iter().collect();
        facts.sort();
        facts.hash(&mut hasher);
        hasher.finish()
    }

    /// Calculate heuristic (estimated cost to reach goal)
    fn calculate_heuristic(&self, from: &WorldState, to: &WorldState) -> f32 {
        // Count missing facts
        let mut missing = 0;
        for fact in &to.facts {
            if !from.has(fact) {
                missing += 1;
            }
        }
        missing as f32 * 5.0 // Approximate cost per missing fact
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goap_planning() {
        let actions = vec![
            ActionDefinition {
                id: "eat".to_string(),
                name: "Eat".to_string(),
                base_cost: 1.0,
                intended_use: 95,
                required_skill: None,
                preconditions: vec!["HasFood".to_string()],
                effects: vec!["NotHungry".to_string()],
            },
            ActionDefinition {
                id: "get_food".to_string(),
                name: "Get Food".to_string(),
                base_cost: 5.0,
                intended_use: 80,
                required_skill: None,
                preconditions: vec![],
                effects: vec!["HasFood".to_string()],
            },
        ];

        let planner = GOAPPlanner::new(actions);

        let mut current_state = WorldState::new();
        // Agent has nothing

        let goal = Goal::new("NotHungry");

        let plan = planner.plan(&current_state, &goal, 100);
        assert!(plan.is_some());
        
        let plan = plan.unwrap();
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0], "get_food");
        assert_eq!(plan[1], "eat");
    }
}

