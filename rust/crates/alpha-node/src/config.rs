use crate::error::NodeError;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SubAccountConfig {
    pub strategy: String,
    pub allocation_pct: Decimal,
    pub symbols: Vec<String>,
    pub timeframe: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SystemConfig {
    pub mode: String,
    pub allow_live_orders: bool,
    pub log_level: String,
    pub database_path: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct UniverseConfig {
    pub venues: Vec<String>,
    pub symbols: Vec<String>,
    pub timeframe: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RiskConfig {
    pub max_risk_per_trade: Decimal,
    pub min_stop_distance: Decimal,
    pub circuit_breaker_drawdown: Decimal,
    pub max_open_positions: usize,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ExecutionConfig {
    pub order_type: String,
    pub maker_fee_bps: Decimal,
    pub taker_fee_bps: Decimal,
    pub slippage_allowance_bps: Decimal,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct MonitoringConfig {
    pub heartbeat_interval_secs: u64,
    pub deadman_interval_secs: u64,
    pub deadman_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub system: SystemConfig,
    pub universe: UniverseConfig,
    pub risk: RiskConfig,
    pub execution: ExecutionConfig,
    pub monitoring: MonitoringConfig,
    #[serde(default)]
    pub sub_accounts: HashMap<String, SubAccountConfig>,
}

impl AppConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, NodeError> {
        let content = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), NodeError> {
        self.validate_system()?;
        self.validate_risk()?;
        Ok(())
    }

    fn validate_system(&self) -> Result<(), NodeError> {
        if self.system.mode == "paper" && self.system.allow_live_orders {
            return Err(NodeError::ConfigError {
                reason: "hard safety violation: allow_live_orders must be false in paper mode"
                    .to_string(),
            });
        }
        if self.universe.symbols.is_empty() {
            return Err(NodeError::ConfigError {
                reason: "universe symbols list cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    fn validate_risk(&self) -> Result<(), NodeError> {
        use rust_decimal_macros::dec;
        if self.risk.max_risk_per_trade <= dec!(0) || self.risk.max_risk_per_trade > dec!(0.05) {
            return Err(NodeError::ConfigError {
                reason: "max_risk_per_trade must be between 0 and 0.05 (5%)".to_string(),
            });
        }
        if self.risk.min_stop_distance < dec!(0.022) {
            return Err(NodeError::ConfigError {
                reason: "min_stop_distance must be >= 0.022 (2.20% structural floor)".to_string(),
            });
        }
        if self.risk.circuit_breaker_drawdown <= dec!(0)
            || self.risk.circuit_breaker_drawdown > dec!(0.20)
        {
            return Err(NodeError::ConfigError {
                reason: "circuit_breaker_drawdown must be between 0 and 0.20 (20%)".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_load_paper_config() {
        let config = AppConfig::load_from_file("../../../configs/paper.toml").unwrap();
        assert_eq!(config.system.mode, "paper");
        assert!(!config.system.allow_live_orders);
        assert_eq!(config.universe.symbols.len(), 2);
    }

    #[test]
    fn test_reject_live_orders_in_paper() {
        let toml_str = r#"
        [system]
        mode = "paper"
        allow_live_orders = true
        log_level = "info"
        database_path = "test.db"

        [universe]
        venues = ["okx"]
        symbols = ["BTCUSDT"]
        timeframe = "4h"

        [risk]
        max_risk_per_trade = 0.010
        min_stop_distance = 0.025
        circuit_breaker_drawdown = 0.060
        max_open_positions = 1

        [execution]
        order_type = "post_only"
        maker_fee_bps = 8.0
        taker_fee_bps = 10.0
        slippage_allowance_bps = 3.0

        [monitoring]
        heartbeat_interval_secs = 60
        deadman_interval_secs = 20
        deadman_timeout_secs = 60
        "#;

        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert!(config.validate().is_err());
    }
}
