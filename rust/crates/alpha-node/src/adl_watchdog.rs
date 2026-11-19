use crate::db::DbManager;
use crate::error::NodeError;
use crate::telegram::TelegramNotifier;
use alpha_core::carry::check_adl_desync;
use alpha_types::order::{OrderIntent, OrderType, Side, Venue};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub const DEFAULT_ADL_TOLERANCE: Decimal = dec!(0.99);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdlWatchdogResult {
    pub is_desynced: bool,
    pub symbol: String,
    pub spot_qty: Decimal,
    pub perp_qty: Decimal,
}

#[must_use]
pub fn check_position_adl_desync(
    symbol: &str,
    spot_qty: Decimal,
    perp_qty: Decimal,
    tolerance: Option<Decimal>,
) -> AdlWatchdogResult {
    let tol = tolerance.unwrap_or(DEFAULT_ADL_TOLERANCE);
    let desynced = check_adl_desync(spot_qty, perp_qty, tol);
    AdlWatchdogResult {
        is_desynced: desynced,
        symbol: symbol.to_string(),
        spot_qty,
        perp_qty,
    }
}

pub fn build_emergency_spot_close_intent(
    venue: Venue,
    symbol: &str,
    spot_qty: Decimal,
    now_ms: i64,
) -> Result<OrderIntent, NodeError> {
    if spot_qty <= Decimal::ZERO {
        return Err(NodeError::StateError {
            reason: format!("cannot close non-positive spot quantity: {}", spot_qty),
        });
    }
    let client_id = format!("ADL_CLOSE_{}_{}", symbol, now_ms);
    let intent = OrderIntent::new(
        venue,
        symbol,
        Side::Sell,
        OrderType::Market,
        None,
        spot_qty,
        None,
        None,
        client_id,
    )?;
    Ok(intent)
}

pub fn handle_adl_desync_trigger(
    db: &DbManager,
    telegram: &TelegramNotifier,
    symbol: &str,
    spot_qty: Decimal,
    perp_qty: Decimal,
    now_ms: i64,
) -> Result<Option<OrderIntent>, NodeError> {
    let reason = format!(
        "HALTED_ADL_DESYNC: symbol={} spot_qty={} perp_qty={}",
        symbol, spot_qty, perp_qty
    );
    db.set_halt_state(true, Some(&reason), now_ms)?;
    telegram.notify_adl_desync(symbol, spot_qty, perp_qty);

    if spot_qty > Decimal::ZERO {
        let intent = build_emergency_spot_close_intent(Venue::Binance, symbol, spot_qty, now_ms)?;
        Ok(Some(intent))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_adl_watchdog_detects_and_persists_halt_with_close_order() {
        let db = DbManager::open_in_memory().unwrap();
        let tg = TelegramNotifier::new(None, None);

        let res = check_position_adl_desync("BTCUSDT", dec!(1.0), dec!(-0.95), None);
        assert!(res.is_desynced);

        let intent =
            handle_adl_desync_trigger(&db, &tg, &res.symbol, res.spot_qty, res.perp_qty, 5000)
                .unwrap();

        let (halted, reason) = db.get_halt_state().unwrap();
        assert!(halted);
        assert!(reason.unwrap().contains("HALTED_ADL_DESYNC"));

        let order = intent.unwrap();
        assert_eq!(order.side, Side::Sell);
        assert_eq!(order.qty, dec!(1.0));
        assert_eq!(order.order_type, OrderType::Market);
    }
}
