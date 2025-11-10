/// Agent Layer - Individual agent definitions and behavior
pub mod agent;
pub mod lifecycle;
pub mod skills;
pub mod personality;
pub mod ownership;

pub use agent::{SimAgent, AgentState, Job, SocialClass, BuildingResources};
pub use lifecycle::*;
pub use skills::*;
pub use personality::*;
pub use ownership::*;

