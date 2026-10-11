//! Deterministic indicators calculated from closes, oldest first.
//! Warm-up values are absent, not fabricated zeros. EMA starts with an SMA seed.
pub(crate) type Series = Vec<Option<f64>>;

pub(crate) fn sma(values: &[f64], period: usize) -> Series {
    let mut out = vec![None; values.len()];
    if period == 0 || period > values.len() || values.iter().any(|v| !v.is_finite()) {
        return out;
    }
    for end in period - 1..values.len() {
        out[end] = Some(values[end + 1 - period..=end].iter().sum::<f64>() / period as f64);
    }
    out
}

pub(crate) fn ema(values: &[f64], period: usize) -> Series {
    let mut out = vec![None; values.len()];
    if period == 0 || period > values.len() || values.iter().any(|v| !v.is_finite()) {
        return out;
    }
    let mut value = values[..period].iter().sum::<f64>() / period as f64;
    out[period - 1] = Some(value);
    let alpha = 2.0 / (period as f64 + 1.0);
    for i in period..values.len() {
        value += (values[i] - value) * alpha;
        out[i] = Some(value);
    }
    out
}

pub(crate) fn bollinger(
    values: &[f64],
    period: usize,
    deviations: f64,
) -> (Series, Series, Series) {
    let middle = sma(values, period);
    let mut upper = vec![None; values.len()];
    let mut lower = upper.clone();
    if !deviations.is_finite() || deviations < 0.0 {
        return (upper, middle, lower);
    }
    for (i, mean) in middle.iter().enumerate() {
        if let Some(mean) = mean {
            let variance = values[i + 1 - period..=i]
                .iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>()
                / period as f64;
            let width = variance.sqrt() * deviations;
            upper[i] = Some(mean + width);
            lower[i] = Some(mean - width);
        }
    }
    (upper, middle, lower)
}

pub(crate) fn rsi(values: &[f64], period: usize) -> Series {
    let mut out = vec![None; values.len()];
    if period == 0 || values.len() <= period || values.iter().any(|v| !v.is_finite()) {
        return out;
    }
    let mut gain = 0.0;
    let mut loss = 0.0;
    for i in 1..=period {
        let delta = values[i] - values[i - 1];
        gain += delta.max(0.0);
        loss += (-delta).max(0.0);
    }
    gain /= period as f64;
    loss /= period as f64;
    let strength = |g: f64, l: f64| {
        if g + l == 0.0 {
            0.0
        } else {
            100.0 * g / (g + l)
        }
    };
    out[period] = Some(strength(gain, loss));
    for i in period + 1..values.len() {
        let delta = values[i] - values[i - 1];
        gain = (gain * (period - 1) as f64 + delta.max(0.0)) / period as f64;
        loss = (loss * (period - 1) as f64 + (-delta).max(0.0)) / period as f64;
        out[i] = Some(strength(gain, loss));
    }
    out
}

pub(crate) fn macd(
    values: &[f64],
    fast: usize,
    slow: usize,
    signal_period: usize,
) -> (Series, Series, Series) {
    let fast_ma = ema(values, fast);
    let slow_ma = ema(values, slow);
    let mut line = vec![None; values.len()];
    let mut signal = line.clone();
    let mut histogram = line.clone();
    if fast == 0 || slow == 0 || fast >= slow || signal_period == 0 {
        return (line, signal, histogram);
    }
    for i in 0..values.len() {
        line[i] = fast_ma[i].zip(slow_ma[i]).map(|(f, s)| f - s);
    }
    if let Some(start) = line.iter().position(Option::is_some) {
        let compact: Vec<_> = line[start..].iter().filter_map(|v| *v).collect();
        let smoothed = ema(&compact, signal_period);
        for (offset, s) in smoothed.into_iter().enumerate() {
            signal[start + offset] = s;
            histogram[start + offset] = line[start + offset].zip(s).map(|(m, s)| m - s);
        }
    }
    (line, signal, histogram)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-8, "{a} != {b}");
    }
    #[test]
    fn averages_have_sma_seed_and_warmup() {
        assert_eq!(
            sma(&[1., 2., 3., 4., 5.], 3),
            vec![None, None, Some(2.), Some(3.), Some(4.)]
        );
        assert_eq!(
            ema(&[1., 2., 3., 4., 5.], 3),
            vec![None, None, Some(2.), Some(3.), Some(4.)]
        );
        assert!(ema(&[1., 2.], 3).iter().all(Option::is_none));
    }
    #[test]
    fn wilder_rsi_matches_reference_example() {
        let closes = [
            44.34, 44.09, 44.15, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08, 45.89, 46.03,
            45.61, 46.28, 46.28, 46.00,
        ];
        let result = rsi(&closes, 14);
        assert!(result[..14].iter().all(Option::is_none));
        near(result[14].unwrap(), 70.46413502109705);
        near(result[15].unwrap(), 66.24961855355505);
        near(rsi(&[1., 2., 3., 4.], 3)[3].unwrap(), 100.);
        near(rsi(&[4., 3., 2., 1.], 3)[3].unwrap(), 0.);
        near(rsi(&[1e-20, 2e-20, 3e-20, 4e-20], 3)[3].unwrap(), 100.);
    }
    #[test]
    fn bollinger_uses_population_deviation() {
        let (upper, middle, lower) = bollinger(&[1., 2., 3., 4., 5.], 5, 2.);
        near(middle[4].unwrap(), 3.);
        near(upper[4].unwrap(), 3. + 2. * 2.0_f64.sqrt());
        near(lower[4].unwrap(), 3. - 2. * 2.0_f64.sqrt());
    }
    #[test]
    fn macd_linear_trend_and_signal_warmup() {
        let closes: Vec<_> = (1..=80).map(|v| v as f64).collect();
        let (line, signal, hist) = macd(&closes, 12, 26, 9);
        assert!(line[..25].iter().all(Option::is_none));
        assert!(signal[..33].iter().all(Option::is_none));
        near(line[25].unwrap(), 7.);
        near(signal[33].unwrap(), 7.);
        near(hist[33].unwrap(), 0.);
    }
    #[test]
    fn invalid_inputs_never_create_indicator_values() {
        assert!(sma(&[1., f64::NAN], 1).iter().all(Option::is_none));
        assert!(rsi(&[1., 2.], 0).iter().all(Option::is_none));
        assert!(macd(&[1., 2.], 2, 1, 1).0.iter().all(Option::is_none));
    }
}
