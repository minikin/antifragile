//! Pricing calculation logic
//!
//! This module simulates complex pricing calculations that benefit from caching.

use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::time::Duration;

/// A price query representing a product configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceQuery {
    pub product_id: String,
    pub quantity: u32,
    pub options: Vec<String>,
}

impl PartialEq for PriceQuery {
    fn eq(&self, other: &Self) -> bool {
        self.product_id == other.product_id
            && self.quantity == other.quantity
            && self.options == other.options
    }
}

impl Eq for PriceQuery {}

impl Hash for PriceQuery {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.product_id.hash(state);
        self.quantity.hash(state);
        for opt in &self.options {
            opt.hash(state);
        }
    }
}

/// Result of a price calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceResult {
    pub base_price: f64,
    pub quantity_discount: f64,
    pub options_cost: f64,
    pub total_price: f64,
}

/// Base prices for different products
fn get_base_price(product_id: &str) -> f64 {
    match product_id {
        "widget-001" => 10.00,
        "widget-002" => 15.00,
        "gadget-001" => 25.00,
        "gadget-002" => 35.00,
        "premium-001" => 100.00,
        "premium-002" => 150.00,
        _ => 20.00, // Default price
    }
}

/// Calculate quantity discount (volume pricing)
fn calculate_quantity_discount(quantity: u32) -> f64 {
    match quantity {
        0..=10 => 0.0,
        11..=50 => 0.05,   // 5% discount
        51..=100 => 0.10,  // 10% discount
        101..=500 => 0.15, // 15% discount
        _ => 0.20,         // 20% discount for bulk
    }
}

/// Calculate additional cost for options
fn calculate_options_cost(options: &[String], base_price: f64) -> f64 {
    let mut cost = 0.0;

    for option in options {
        cost += match option.as_str() {
            "express-shipping" => 5.00 + base_price * 0.02,
            "gift-wrap" => 3.00,
            "insurance" => base_price * 0.05,
            "priority-support" => 10.00,
            "extended-warranty" => base_price * 0.15,
            _ => 0.0,
        };
    }

    cost
}

/// Simulate complex pricing calculation
///
/// This function intentionally includes a small delay to simulate
/// a computationally expensive operation (e.g., calling external APIs,
/// complex business rules, database lookups).
///
/// The key insight: when this is cached, the system becomes antifragile
/// because repeated queries (higher load) result in faster responses.
pub async fn calculate_price(query: &PriceQuery) -> PriceResult {
    // Simulate computation time (5-15ms)
    // In a real system, this might be database queries, API calls, etc.
    let computation_delay = Duration::from_millis(5 + (query.product_id.len() as u64 % 10));
    tokio::time::sleep(computation_delay).await;

    let base_price = get_base_price(&query.product_id);
    let subtotal = base_price * query.quantity as f64;

    let discount_rate = calculate_quantity_discount(query.quantity);
    let quantity_discount = subtotal * discount_rate;

    let options_cost = calculate_options_cost(&query.options, base_price) * query.quantity as f64;

    let total_price = subtotal - quantity_discount + options_cost;

    PriceResult {
        base_price,
        quantity_discount,
        options_cost,
        total_price: (total_price * 100.0).round() / 100.0, // Round to cents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_base_pricing() {
        let query = PriceQuery {
            product_id: "widget-001".to_string(),
            quantity: 1,
            options: vec![],
        };

        let result = calculate_price(&query).await;
        assert!((result.total_price - 10.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_quantity_discount() {
        let query = PriceQuery {
            product_id: "widget-001".to_string(),
            quantity: 100,
            options: vec![],
        };

        let result = calculate_price(&query).await;
        // 100 * $10 = $1000, 10% discount = $100 off = $900
        assert!((result.total_price - 900.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_options_pricing() {
        let query = PriceQuery {
            product_id: "widget-001".to_string(),
            quantity: 1,
            options: vec!["gift-wrap".to_string()],
        };

        let result = calculate_price(&query).await;
        // $10 base + $3 gift wrap = $13
        assert!((result.total_price - 13.0).abs() < 0.01);
    }

    #[test]
    fn test_query_equality() {
        let q1 = PriceQuery {
            product_id: "widget-001".to_string(),
            quantity: 10,
            options: vec!["gift-wrap".to_string()],
        };

        let q2 = PriceQuery {
            product_id: "widget-001".to_string(),
            quantity: 10,
            options: vec!["gift-wrap".to_string()],
        };

        assert_eq!(q1, q2);
    }
}
