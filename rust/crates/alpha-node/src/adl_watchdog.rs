use crate::db::DbManager;
use crate::error::NodeError;
use crate::telegram::TelegramNotifier;
use alpha_core::carry::check_adl_desync;
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

pub fn handle_adl_desync_trigger(
    db: &DbManager,
    telegram: &TelegramNotifier,
    symbol: &str,
    spot_qty: Decimal,
    perp_qty: Decimal,
    now_ms: i64,
) -> Result<(), NodeError> {
    let reason = format!(
        "HALTED_ADL_DESYNC: symbol={} spot_qty={} perp_qty={}",
        symbol, spot_qty, perp_qty
    );
    db.set_halt_state(true, Some(&reason), now_ms)?;
    telegram.notify_adl_desync(symbol, spot_qty, perp_qty);
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_adl_watchdog_detects_and_persists_halt() {
        let db = DbManager::open_in_memory().unwrap();
        let tg = TelegramNotifier::new(None, None);

        let res = check_position_adl_desync("BTCUSDT", dec!(1.0), dec!(-0.95), None);
        assert!(res.is_desynced);

        handle_adl_desync_trigger(&db, &tg, &res.symbol, res.spot_qty, res.perp_qty, 5000).unwrap();

        let (halted, reason) = db.get_halt_state().unwrap();
        assert!(halted);
        assert!(reason.unwrap().contains("HALTED_ADL_DESYNC"));
    }
}
