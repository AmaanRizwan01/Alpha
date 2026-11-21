use crate::db::{DbManager, PositionRecord};
use crate::error::NodeError;
use rust_decimal::Decimal;

#[derive(Debug, PartialEq, Eq)]
pub enum ReconcileOutcome {
    Synchronized,
    PositionClosedOffline { symbol: String },
    UnhedgedPositionDetected { symbol: String, qty: Decimal },
    Halted { reason: String },
}

pub fn reconcile_cold_boot(
    db: &DbManager,
    symbol: &str,
    venue_position: Option<&PositionRecord>,
    venue_has_stop: bool,
    now_ms: i64,
) -> Result<ReconcileOutcome, NodeError> {
    let (is_halted, halt_reason) = db.get_halt_state()?;
    if is_halted {
        return Ok(ReconcileOutcome::Halted {
            reason: halt_reason.unwrap_or_else(|| "unspecified halt".into()),
        });
    }

    let local_pos = db.get_position(symbol)?;

    match (local_pos, venue_position) {
        // Both flat: completely synchronized
        (None, None) => Ok(ReconcileOutcome::Synchronized),

        // Local recorded open, but venue has no position -> stopped out offline
        (Some(_), None) => {
            db.delete_position(symbol)?;
            Ok(ReconcileOutcome::PositionClosedOffline {
                symbol: symbol.to_string(),
            })
        }

        // Venue has position, but local recorded none -> unhedged rogue position
        (None, Some(v_pos)) => {
            db.set_halt_state(
                true,
                Some("unhedged exchange position without local record"),
                now_ms,
            )?;
            Ok(ReconcileOutcome::UnhedgedPositionDetected {
                symbol: symbol.to_string(),
                qty: v_pos.qty,
            })
        }

        // Both hold position: verify exchange-side protective stop exists
        (Some(l_pos), Some(v_pos)) => {
            if !venue_has_stop {
                db.set_halt_state(true, Some("active position missing exchange stop"), now_ms)?;
                Ok(ReconcileOutcome::UnhedgedPositionDetected {
                    symbol: symbol.to_string(),
                    qty: v_pos.qty,
                })
            } else {
                // Keep local high-water mark of position price updated
                let highest = l_pos.highest_price.max(v_pos.highest_price);
                db.upsert_position(
                    symbol,
                    &l_pos.venue,
                    v_pos.qty,
                    l_pos.entry_price,
                    l_pos.current_stop_price,
                    highest,
                    l_pos.breakeven_active,
                    l_pos.bars_held,
                    now_ms,
                )?;
                Ok(ReconcileOutcome::Synchronized)
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_reconcile_synchronized_flat() {
        let db = DbManager::open_in_memory().unwrap();
        let outcome = reconcile_cold_boot(&db, "BTCUSDT", None, false, 1000).unwrap();
        assert_eq!(outcome, ReconcileOutcome::Synchronized);
    }

    #[test]
    fn test_reconcile_halted() {
        let db = DbManager::open_in_memory().unwrap();
        db.set_halt_state(true, Some("circuit breaker tripped"), 1000)
            .unwrap();

        let outcome = reconcile_cold_boot(&db, "BTCUSDT", None, false, 2000).unwrap();
        assert!(matches!(outcome, ReconcileOutcome::Halted { .. }));
    }

    #[test]
    fn test_reconcile_position_closed_offline() {
        let db = DbManager::open_in_memory().unwrap();
        db.upsert_position(
            "BTCUSDT",
            "Binance",
            dec!(0.05),
            dec!(65000.0),
            dec!(63500.0),
            dec!(66000.0),
            false,
            2,
            1000,
        )
        .unwrap();

        // Venue returns None (stopped out during restart)
        let outcome = reconcile_cold_boot(&db, "BTCUSDT", None, false, 2000).unwrap();
        assert_eq!(
            outcome,
            ReconcileOutcome::PositionClosedOffline {
                symbol: "BTCUSDT".into()
            }
        );
        // Verify local position cleared
        assert!(db.get_position("BTCUSDT").unwrap().is_none());
    }

    #[test]
    fn test_reconcile_unhedged_position_detected() {
        let db = DbManager::open_in_memory().unwrap();
        let venue_pos = PositionRecord {
            symbol: "BTCUSDT".into(),
            venue: "Binance".into(),
            qty: dec!(0.05),
            entry_price: dec!(65000.0),
            current_stop_price: dec!(63500.0),
            highest_price: dec!(65000.0),
            breakeven_active: false,
            bars_held: 0,
        };

        // Local is None, but Venue has coins
        let outcome = reconcile_cold_boot(&db, "BTCUSDT", Some(&venue_pos), false, 2000).unwrap();
        assert_eq!(
            outcome,
            ReconcileOutcome::UnhedgedPositionDetected {
                symbol: "BTCUSDT".into(),
                qty: dec!(0.05)
            }
        );
        // Verify halt was automatically engaged
        assert!(db.get_halt_state().unwrap().0);
    }
}
