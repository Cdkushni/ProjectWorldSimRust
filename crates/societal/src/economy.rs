use ahash::AHashMap;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;
use world_sim_core::ResourceType;
use world_sim_event_bus::{EventBus, EventSubscriber, PriceChangeEvent, EventEnvelope, BlightStartedEvent};

/// Manages the dynamic economy
pub struct EconomySubsystem {
    prices: Arc<RwLock<AHashMap<ResourceType, f32>>>,
    supply: Arc<RwLock<AHashMap<ResourceType, u32>>>,
    demand: Arc<RwLock<AHashMap<ResourceType, u32>>>,
    event_bus: Arc<EventBus>,
}

impl EconomySubsystem {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        let mut prices = AHashMap::new();
        
        // Initialize default prices
        prices.insert(ResourceType::Wood, 5.0);
        prices.insert(ResourceType::Stone, 3.0);
        prices.insert(ResourceType::Iron, 15.0);
        prices.insert(ResourceType::Gold, 100.0);
        prices.insert(ResourceType::Food, 10.0);
        prices.insert(ResourceType::Water, 1.0);
        prices.insert(ResourceType::Cloth, 8.0);
        prices.insert(ResourceType::Tool, 25.0);
        prices.insert(ResourceType::Weapon, 50.0);
        prices.insert(ResourceType::Coin, 1.0);

        Self {
            prices: Arc::new(RwLock::new(prices)),
            supply: Arc::new(RwLock::new(AHashMap::new())),
            demand: Arc::new(RwLock::new(AHashMap::new())),
            event_bus,
        }
    }

    /// Get current price of a resource
    pub fn get_price(&self, resource: ResourceType) -> f32 {
        *self.prices.read().get(&resource).unwrap_or(&10.0)
    }

    /// Update supply for a resource
    pub fn update_supply(&self, resource: ResourceType, quantity: u32) {
        self.supply.write().insert(resource, quantity);
    }

    /// Update demand for a resource
    pub fn update_demand(&self, resource: ResourceType, quantity: u32) {
        self.demand.write().insert(resource, quantity);
    }

    /// Recalculate prices based on supply and demand
    pub async fn recalculate_prices(&self) {
        // Collect price changes while holding locks
        let price_changes = {
            let supply = self.supply.read();
            let demand = self.demand.read();
            let mut prices = self.prices.write();
            let mut changes = Vec::new();

            for (&resource, &supply_qty) in supply.iter() {
                let demand_qty = demand.get(&resource).copied().unwrap_or(0);
                let old_price = prices.get(&resource).copied().unwrap_or(10.0);

                // Simple supply/demand formula
                let ratio = if supply_qty > 0 {
                    demand_qty as f32 / supply_qty as f32
                } else {
                    2.0 // No supply = high price
                };

                let new_price = old_price * (0.9 + ratio * 0.2);
                let new_price = new_price.clamp(0.1, 1000.0);

                if (new_price - old_price).abs() > 0.5 {
                    changes.push(PriceChangeEvent {
                        resource,
                        old_price,
                        new_price,
                        total_supply: supply_qty,
                        total_demand: demand_qty,
                    });
                }

                prices.insert(resource, new_price);
            }
            
            changes
        }; // All locks dropped here

        // Now publish events without holding any locks
        for change in price_changes {
            self.event_bus.publish(&change).await;
        }
    }

    /// React to a blight event
    pub async fn on_blight_started(&self, resource: ResourceType) {
        // Reduce supply dramatically
        {
            let mut supply = self.supply.write();
            if let Some(qty) = supply.get_mut(&resource) {
                *qty = (*qty as f32 * 0.3) as u32; // 70% reduction
            }
        } // Drop lock before await

        // Recalculate prices
        self.recalculate_prices().await;
    }
}

/// Subscribe to events
#[async_trait]
impl EventSubscriber for EconomySubsystem {
    async fn on_event(&self, event: &EventEnvelope) {
        match event.event_type.as_str() {
            "BlightStarted" => {
                // Parse the event and react
                if let Ok(blight_event) = serde_json::from_value::<BlightStartedEvent>(event.payload.clone()) {
                    self.on_blight_started(blight_event.affected_resource).await;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_economy() {
        let event_bus = Arc::new(EventBus::new());
        let economy = EconomySubsystem::new(event_bus);

        economy.update_supply(ResourceType::Wood, 100);
        economy.update_demand(ResourceType::Wood, 200);

        let initial_price = economy.get_price(ResourceType::Wood);
        economy.recalculate_prices().await;
        let new_price = economy.get_price(ResourceType::Wood);

        assert!(new_price > initial_price); // High demand should increase price
    }
}

