use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use world_sim_core::{Position, ResourceType};

/// A physical market in the world where trade happens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub id: Uuid,
    pub position: Position,
    pub name: String,
    pub market_type: MarketType,
    /// Inventory of goods available for sale
    pub inventory: HashMap<ResourceType, MarketGood>,
    /// Buy orders (agents wanting to purchase)
    pub buy_orders: Vec<TradeOrder>,
    /// Sell orders (agents wanting to sell)
    pub sell_orders: Vec<TradeOrder>,
    /// Total transactions processed
    pub transaction_count: u64,
    /// Market reputation (0-100, affects prices)
    pub reputation: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketType {
    General,      // Sells everything
    Food,         // Specializes in food and water
    Materials,    // Wood, stone, iron
    Luxury,       // Gold, cloth, expensive items
    Weapons,      // Weapons and tools
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketGood {
    pub resource_type: ResourceType,
    pub quantity: u32,
    pub base_price: f64,
    pub current_price: f64,
    /// Sellers who contributed to this good
    pub sellers: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOrder {
    pub id: Uuid,
    pub agent_id: world_sim_core::AgentId,
    pub resource: ResourceType,
    pub quantity: u32,
    pub price_per_unit: f64,
    pub order_type: OrderType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Buy,
    Sell,
}

impl Market {
    pub fn new(name: String, position: Position, market_type: MarketType) -> Self {
        Self {
            id: Uuid::new_v4(),
            position,
            name,
            market_type,
            inventory: HashMap::new(),
            buy_orders: Vec::new(),
            sell_orders: Vec::new(),
            transaction_count: 0,
            reputation: 50.0,
        }
    }
    
    /// Add goods to market inventory
    pub fn add_inventory(&mut self, resource: ResourceType, quantity: u32, base_price: f64) {
        let good = self.inventory.entry(resource).or_insert(MarketGood {
            resource_type: resource,
            quantity: 0,
            base_price,
            current_price: base_price,
            sellers: Vec::new(),
        });
        good.quantity += quantity;
    }
    
    /// Remove goods from market inventory
    pub fn remove_inventory(&mut self, resource: ResourceType, quantity: u32) -> bool {
        if let Some(good) = self.inventory.get_mut(&resource) {
            if good.quantity >= quantity {
                good.quantity -= quantity;
                return true;
            }
        }
        false
    }
    
    /// Place a buy order
    pub fn place_buy_order(&mut self, order: TradeOrder) {
        self.buy_orders.push(order);
    }
    
    /// Place a sell order
    pub fn place_sell_order(&mut self, order: TradeOrder) {
        self.sell_orders.push(order);
    }
    
    /// Match buy and sell orders
    pub fn match_orders(&mut self) -> Vec<TradeExecution> {
        let mut executions = Vec::new();
        
        // Simple order matching algorithm
        let mut i = 0;
        while i < self.buy_orders.len() {
            let mut j = 0;
            while j < self.sell_orders.len() {
                let buy = &self.buy_orders[i];
                let sell = &self.sell_orders[j];
                
                // Match if same resource and buy price >= sell price
                if buy.resource == sell.resource && buy.price_per_unit >= sell.price_per_unit {
                    let quantity = buy.quantity.min(sell.quantity);
                    let price = (buy.price_per_unit + sell.price_per_unit) / 2.0;
                    
                    executions.push(TradeExecution {
                        id: Uuid::new_v4(),
                        buyer_id: buy.agent_id,
                        seller_id: sell.agent_id,
                        resource: buy.resource,
                        quantity,
                        price_per_unit: price,
                        market_id: self.id,
                    });
                    
                    self.transaction_count += 1;
                    
                    // Update orders
                    let buy_remaining = self.buy_orders[i].quantity - quantity;
                    let sell_remaining = self.sell_orders[j].quantity - quantity;
                    
                    if buy_remaining == 0 {
                        self.buy_orders.remove(i);
                        // Don't increment i, check same index again
                    } else {
                        self.buy_orders[i].quantity = buy_remaining;
                        i += 1;
                    }
                    
                    if sell_remaining == 0 {
                        self.sell_orders.remove(j);
                    } else {
                        self.sell_orders[j].quantity = sell_remaining;
                        j += 1;
                    }
                    
                    break; // Move to next buy order
                } else {
                    j += 1;
                }
            }
            
            // If no match found, move to next buy order
            if j >= self.sell_orders.len() {
                i += 1;
            }
        }
        
        executions
    }
    
    /// Update prices based on supply and demand
    pub fn update_prices(&mut self) {
        for good in self.inventory.values_mut() {
            // Calculate demand from buy orders
            let demand: u32 = self.buy_orders
                .iter()
                .filter(|o| o.resource == good.resource_type)
                .map(|o| o.quantity)
                .sum();
            
            // Price adjustment based on inventory and demand
            let supply_factor = if good.quantity > 0 {
                demand as f64 / good.quantity as f64
            } else {
                2.0
            };
            
            good.current_price = good.base_price * (0.8 + supply_factor * 0.4);
            good.current_price = good.current_price.clamp(good.base_price * 0.5, good.base_price * 3.0);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecution {
    pub id: Uuid,
    pub buyer_id: world_sim_core::AgentId,
    pub seller_id: world_sim_core::AgentId,
    pub resource: ResourceType,
    pub quantity: u32,
    pub price_per_unit: f64,
    pub market_id: Uuid,
}

/// Manager for all markets in the world
pub struct MarketSystem {
    markets: AHashMap<Uuid, Market>,
}

impl MarketSystem {
    pub fn new() -> Self {
        Self {
            markets: AHashMap::new(),
        }
    }
    
    pub fn create_market(&mut self, name: String, position: Position, market_type: MarketType) -> Uuid {
        let market = Market::new(name, position, market_type);
        let id = market.id;
        self.markets.insert(id, market);
        id
    }
    
    pub fn get_market(&self, id: Uuid) -> Option<&Market> {
        self.markets.get(&id)
    }
    
    pub fn get_market_mut(&mut self, id: Uuid) -> Option<&mut Market> {
        self.markets.get_mut(&id)
    }
    
    pub fn get_all_markets(&self) -> Vec<&Market> {
        self.markets.values().collect()
    }
    
    pub fn get_all_markets_mut(&mut self) -> Vec<&mut Market> {
        self.markets.values_mut().collect()
    }
    
    pub fn find_nearest_market(&self, position: &Position, market_type: Option<MarketType>) -> Option<&Market> {
        self.markets
            .values()
            .filter(|m| market_type.is_none() || market_type == Some(m.market_type))
            .min_by(|a, b| {
                let dist_a = a.position.distance_to(position);
                let dist_b = b.position.distance_to(position);
                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            })
    }
    
    /// Process all market orders
    pub fn process_all_markets(&mut self) -> Vec<TradeExecution> {
        let mut all_executions = Vec::new();
        
        for market in self.markets.values_mut() {
            market.update_prices();
            let executions = market.match_orders();
            all_executions.extend(executions);
        }
        
        all_executions
    }
}

impl Default for MarketSystem {
    fn default() -> Self {
        Self::new()
    }
}

