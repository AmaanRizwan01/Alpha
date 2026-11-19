use crate::error::VenueError;
use crate::signer::sign_binance;
use alpha_types::{OrderIntent, OrderType, Side};
use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinanceOtocoParams {
    pub symbol: String,
    pub working_side: String,
    pub working_type: String,
    pub working_price: Decimal,
    pub working_quantity: Decimal,
    pub working_client_order_id: String,
    pub pending_side: String,
    pub pending_quantity: Decimal,
    pub pending_below_type: String,
    pub pending_below_stop_price: Decimal,
    pub pending_below_price: Decimal,
    pub pending_below_client_order_id: String,
    pub pending_above_type: Option<String>,
    pub pending_above_price: Option<Decimal>,
    pub pending_above_client_order_id: Option<String>,
}

pub fn build_binance_atomic_otoco(
    intent: &OrderIntent,
    stop_limit_buffer_bps: u32,
) -> Result<BinanceOtocoParams, VenueError> {
    let working_price = intent.price.ok_or_else(|| VenueError::InvalidOrder {
        reason: "Binance OTOCO working order requires a limit price".into(),
    })?;

    let stop_price = intent
        .stop_loss_price
        .ok_or_else(|| VenueError::InvalidOrder {
            reason: "Binance OTOCO requires a stop loss price".into(),
        })?;

    let (working_side, pending_side) = match intent.side {
        Side::Buy => ("BUY", "SELL"),
        Side::Sell => ("SELL", "BUY"),
    };

    let working_type = match intent.order_type {
        OrderType::LimitMaker => "LIMIT_MAKER",
        OrderType::Limit => "LIMIT",
        _ => {
            return Err(VenueError::InvalidOrder {
                reason: "Binance OTOCO working order must be LIMIT or LIMIT_MAKER".into(),
            });
        }
    };

    // Calculate slippage collar on the stop limit price
    // for buy (long stop is sell): stop_limit_price = stop_price * (1 - buffer_bps / 10000)
    let buffer_fraction = Decimal::from(stop_limit_buffer_bps) / Decimal::from(10000);
    let pending_below_price = match intent.side {
        Side::Buy => (stop_price * (Decimal::ONE - buffer_fraction)).round_dp(2),
        Side::Sell => (stop_price * (Decimal::ONE + buffer_fraction)).round_dp(2),
    };

    let mut pending_above_type = None;
    let mut pending_above_price = None;
    let mut pending_above_client_order_id = None;

    if let Some(tp) = intent.take_profit_price {
        pending_above_type = Some("LIMIT_MAKER".into());
        pending_above_price = Some(tp);
        pending_above_client_order_id = Some(format!("tp-{}", intent.client_order_id));
    }

    Ok(BinanceOtocoParams {
        symbol: intent.symbol.clone(),
        working_side: working_side.into(),
        working_type: working_type.into(),
        working_price,
        working_quantity: intent.qty,
        working_client_order_id: intent.client_order_id.clone(),
        pending_side: pending_side.into(),
        pending_quantity: intent.qty,
        pending_below_type: "STOP_LOSS_LIMIT".into(),
        pending_below_stop_price: stop_price,
        pending_below_price,
        pending_below_client_order_id: format!("sl-{}", intent.client_order_id),
        pending_above_type,
        pending_above_price,
        pending_above_client_order_id,
    })
}

impl BinanceOtocoParams {
    pub fn to_query_string(&self, timestamp: u64, recv_window: u64) -> String {
        let mut query = format!(
            "symbol={}&workingSide={}&workingType={}&workingPrice={}&workingQuantity={}&workingClientOrderId={}&pendingSide={}&pendingQuantity={}&pendingBelowType={}&pendingBelowStopPrice={}&pendingBelowPrice={}&pendingBelowClientOrderId={}&recvWindow={}&timestamp={}",
            self.symbol,
            self.working_side,
            self.working_type,
            self.working_price,
            self.working_quantity,
            self.working_client_order_id,
            self.pending_side,
            self.pending_quantity,
            self.pending_below_type,
            self.pending_below_stop_price,
            self.pending_below_price,
            self.pending_below_client_order_id,
            recv_window,
            timestamp
        );

        if let (Some(above_type), Some(above_px), Some(above_cl_id)) = (
            &self.pending_above_type,
            &self.pending_above_price,
            &self.pending_above_client_order_id,
        ) {
            query.push_str(&format!(
                "&pendingAboveType={}&pendingAbovePrice={}&pendingAboveClientOrderId={}",
                above_type, above_px, above_cl_id
            ));
        }

        query
    }

    pub fn to_signed_query(
        &self,
        secret: &str,
        timestamp: u64,
        recv_window: u64,
    ) -> Result<String, VenueError> {
        let base_query = self.to_query_string(timestamp, recv_window);
        let signature = sign_binance(secret, &base_query)?;
        Ok(format!("{}&signature={}", base_query, signature))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use alpha_types::Venue;
    use rust_decimal_macros::dec;

    #[test]
    fn test_binance_otoco_params_construction() {
        let intent = OrderIntent::new(
            Venue::Binance,
            "BTCUSDT",
            Side::Buy,
            OrderType::LimitMaker,
            Some(dec!(65000.0)),
            dec!(0.05),
            Some(dec!(63500.0)),
            Some(dec!(70000.0)),
            "ord-binance-001",
        )
        .unwrap();

        // 10 bps slippage collar on stop limit execution
        let otoco = build_binance_atomic_otoco(&intent, 10).unwrap();
        assert_eq!(otoco.symbol, "BTCUSDT");
        assert_eq!(otoco.working_side, "BUY");
        assert_eq!(otoco.working_type, "LIMIT_MAKER");
        assert_eq!(otoco.pending_side, "SELL");
        assert_eq!(otoco.pending_below_type, "STOP_LOSS_LIMIT");
        assert_eq!(otoco.pending_below_stop_price, dec!(63500.0));
        // 63500 * (1 - 0.001) = 63436.5
        assert_eq!(otoco.pending_below_price, dec!(63436.5));
        assert_eq!(otoco.pending_above_type.as_deref(), Some("LIMIT_MAKER"));
        assert_eq!(otoco.pending_above_price, Some(dec!(70000.0)));

        let query = otoco.to_query_string(1700000000000, 5000);
        assert!(query.contains("workingSide=BUY"));
        assert!(query.contains("pendingBelowStopPrice=63500.0"));
        assert!(query.contains("pendingAbovePrice=70000.0"));

        let signed = otoco
            .to_signed_query("test-secret", 1700000000000, 5000)
            .unwrap();
        assert!(signed.contains("&signature="));
    }
}
