use crate::error::NodeError;
use alpha_types::Bar;

#[derive(Debug, Clone)]
pub struct BarBuffer {
    capacity: usize,
    bars: Vec<Bar>,
}

impl BarBuffer {
    pub fn new(capacity: usize) -> Result<Self, NodeError> {
        if capacity == 0 {
            return Err(NodeError::BufferError {
                reason: "capacity must be strictly positive".to_string(),
            });
        }
        Ok(Self {
            capacity,
            bars: Vec::with_capacity(capacity),
        })
    }

    pub fn from_bars(capacity: usize, initial_bars: Vec<Bar>) -> Result<Self, NodeError> {
        let mut buffer = Self::new(capacity)?;
        for bar in initial_bars {
            buffer.push(bar)?;
        }
        Ok(buffer)
    }

    pub fn push(&mut self, bar: Bar) -> Result<(), NodeError> {
        if let Some(last) = self.bars.last() {
            if bar.open_time_ms < last.open_time_ms {
                return Err(NodeError::BufferError {
                    reason: format!(
                        "out-of-order bar: new bar open_time {} < last open_time {}",
                        bar.open_time_ms, last.open_time_ms
                    ),
                });
            }
            if bar.open_time_ms == last.open_time_ms {
                if let Some(last_mut) = self.bars.last_mut() {
                    *last_mut = bar;
                }
                return Ok(());
            }
        }

        if self.bars.len() >= self.capacity && !self.bars.is_empty() {
            self.bars.remove(0);
        }
        self.bars.push(bar);
        Ok(())
    }

    #[must_use]
    pub fn bars(&self) -> &[Bar] {
        &self.bars
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bars.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[must_use]
    pub fn latest(&self) -> Option<&Bar> {
        self.bars.last()
    }

    #[must_use]
    pub fn is_ready(&self, min_bars: usize) -> bool {
        self.bars.len() >= min_bars
    }

    pub fn clear(&mut self) {
        self.bars.clear();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_test_bar(open_time_ms: i64, close_px: rust_decimal::Decimal) -> Bar {
        Bar::new(
            open_time_ms,
            open_time_ms + 3_600_000,
            close_px,
            close_px + dec!(100.0),
            close_px - dec!(50.0),
            close_px,
            dec!(1000.0),
            dec!(600.0),
            dec!(50_000_000.0),
            500,
        )
        .unwrap()
    }

    #[test]
    fn test_zero_capacity_rejected() {
        let res = BarBuffer::new(0);
        assert!(res.is_err());
    }

    #[test]
    fn test_buffer_push_and_fifo_clamp() {
        let mut buffer = BarBuffer::new(3).unwrap();
        assert_eq!(buffer.capacity(), 3);
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);

        buffer.push(make_test_bar(1000, dec!(100.0))).unwrap();
        buffer.push(make_test_bar(2000, dec!(105.0))).unwrap();
        buffer.push(make_test_bar(3000, dec!(110.0))).unwrap();
        assert_eq!(buffer.len(), 3);
        assert!(buffer.is_ready(3));
        assert!(!buffer.is_ready(4));

        // Push 4th bar -> oldest (1000) dropped, now [2000, 3000, 4000]
        buffer.push(make_test_bar(4000, dec!(115.0))).unwrap();
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.bars().first().unwrap().open_time_ms, 2000);
        assert_eq!(buffer.latest().unwrap().open_time_ms, 4000);
    }

    #[test]
    fn test_buffer_deduplication() {
        let mut buffer = BarBuffer::new(5).unwrap();
        buffer.push(make_test_bar(1000, dec!(100.0))).unwrap();
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.latest().unwrap().close, dec!(100.0));

        // Push bar with same open_time_ms but updated close
        buffer.push(make_test_bar(1000, dec!(102.5))).unwrap();
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.latest().unwrap().close, dec!(102.5));
    }

    #[test]
    fn test_buffer_out_of_order_rejected() {
        let mut buffer = BarBuffer::new(5).unwrap();
        buffer.push(make_test_bar(2000, dec!(100.0))).unwrap();

        // Attempt to push bar with older timestamp
        let err = buffer.push(make_test_bar(1000, dec!(95.0)));
        assert!(err.is_err());
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_buffer_from_bars() {
        let bars = vec![
            make_test_bar(1000, dec!(100.0)),
            make_test_bar(2000, dec!(102.0)),
            make_test_bar(3000, dec!(104.0)),
        ];
        let buffer = BarBuffer::from_bars(2, bars).unwrap();
        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.bars().first().unwrap().open_time_ms, 2000);
        assert_eq!(buffer.latest().unwrap().open_time_ms, 3000);
    }
}
