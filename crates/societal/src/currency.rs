use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

/// Currency system with inflation tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencySystem {
    /// Total money supply in the world
    pub total_supply: f64,
    /// Base currency value (tracks inflation)
    pub base_value: f64,
    /// Inflation rate (percentage per game year)
    pub inflation_rate: f64,
    /// Deflation events counter
    pub deflation_events: u32,
    /// Transaction count (for velocity of money calculation)
    pub transaction_count: u64,
}

impl Default for CurrencySystem {
    fn default() -> Self {
        Self {
            total_supply: 10000.0, // Starting money supply
            base_value: 1.0,
            inflation_rate: 0.0,
            deflation_events: 0,
            transaction_count: 0,
        }
    }
}

impl CurrencySystem {
    pub fn new(initial_supply: f64) -> Self {
        Self {
            total_supply: initial_supply,
            ..Default::default()
        }
    }
    
    /// Inject new currency (from mining, trading, etc.)
    pub fn mint_currency(&mut self, amount: f64) {
        self.total_supply += amount;
        self.recalculate_inflation();
    }
    
    /// Remove currency from circulation (taxes, destruction, etc.)
    pub fn burn_currency(&mut self, amount: f64) {
        self.total_supply = (self.total_supply - amount).max(0.0);
        self.deflation_events += 1;
        self.recalculate_inflation();
    }
    
    /// Record a transaction
    pub fn record_transaction(&mut self, _amount: f64) {
        self.transaction_count += 1;
    }
    
    /// Calculate current inflation rate based on money supply growth
    fn recalculate_inflation(&mut self) {
        // Simple inflation model: more money supply = higher inflation
        let base_inflation = 0.02; // 2% base
        let supply_factor = (self.total_supply / 10000.0 - 1.0) * 0.05;
        self.inflation_rate = base_inflation + supply_factor;
        
        // Apply inflation to base value
        self.base_value *= 1.0 - (self.inflation_rate * 0.001);
    }
    
    /// Get purchasing power (inverse of inflation)
    pub fn get_purchasing_power(&self) -> f64 {
        self.base_value
    }
    
    /// Get velocity of money (transactions / supply)
    pub fn get_velocity(&self) -> f64 {
        if self.total_supply > 0.0 {
            self.transaction_count as f64 / self.total_supply
        } else {
            0.0
        }
    }
}

/// Agent wallet system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallet {
    pub balance: f64,
    pub total_earned: f64,
    pub total_spent: f64,
}

impl Default for Wallet {
    fn default() -> Self {
        Self {
            balance: 100.0, // Everyone starts with some money
            total_earned: 0.0,
            total_spent: 0.0,
        }
    }
}

impl Wallet {
    pub fn new(initial_balance: f64) -> Self {
        Self {
            balance: initial_balance,
            total_earned: 0.0,
            total_spent: 0.0,
        }
    }
    
    pub fn deposit(&mut self, amount: f64) {
        self.balance += amount;
        self.total_earned += amount;
    }
    
    pub fn withdraw(&mut self, amount: f64) -> bool {
        if self.balance >= amount {
            self.balance -= amount;
            self.total_spent += amount;
            true
        } else {
            false
        }
    }
    
    pub fn can_afford(&self, amount: f64) -> bool {
        self.balance >= amount
    }
}

