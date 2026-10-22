//! Plain statistics over small samples: no allocation tricks, no approximation beyond `f64`.

pub fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

/// Population variance (divided by n), as KR-20 and the point-biserial expect.
pub fn variance(values: &[f64]) -> Option<f64> {
    let m = mean(values)?;
    Some(values.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / values.len() as f64)
}

pub fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mid = sorted.len() / 2;
    Some(if sorted.len().is_multiple_of(2) {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    })
}

/// Pearson correlation; `None` when either side does not vary.
pub fn correlation(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() != y.len() || x.len() < 2 {
        return None;
    }
    let (mx, my) = (mean(x)?, mean(y)?);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        sxy += (a - mx) * (b - my);
        sxx += (a - mx) * (a - mx);
        syy += (b - my) * (b - my);
    }
    (sxx > 0.0 && syy > 0.0).then(|| sxy / (sxx * syy).sqrt())
}

/// Kuder-Richardson 20 over item scores in [0, 1] (`items[student][question]`): reliability of
/// the whole paper. With partial credit it is coefficient alpha, of which KR-20 is the 0/1 case.
pub fn kr20(items: &[Vec<f64>]) -> Option<f64> {
    let k = items.first()?.len();
    if items.len() < 2 || k < 2 || items.iter().any(|row| row.len() != k) {
        return None;
    }
    let totals: Vec<f64> = items.iter().map(|row| row.iter().sum()).collect();
    let total_variance = variance(&totals)?;
    if total_variance <= 0.0 {
        return None;
    }
    let item_variance: f64 = (0..k)
        .map(|q| {
            let column: Vec<f64> = items.iter().map(|row| row[q]).collect();
            variance(&column).unwrap_or(0.0)
        })
        .sum();
    let k = k as f64;
    Some(k / (k - 1.0) * (1.0 - item_variance / total_variance))
}

/// P(X >= k) for X binomial with `n` trials of probability `p`: how often chance alone gives
/// at least `k` successes.
pub fn binomial_tail(n: u32, k: u32, p: f64) -> f64 {
    if k == 0 {
        return 1.0;
    }
    if k > n || p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    let ratio = p / (1.0 - p);
    let mut pmf = (1.0 - p).powi(n as i32);
    let mut below = 0.0;
    for i in 0..k {
        below += pmf;
        pmf *= f64::from(n - i) / f64::from(i + 1) * ratio;
    }
    (1.0 - below).clamp(0.0, 1.0)
}

/// Indices of the lower and upper groups: the `fraction` of students (rounded down, at least
/// one) with the lowest and the highest totals. Ties at a boundary keep the input order, so the
/// result is reproducible.
pub fn extreme_groups(totals: &[f64], fraction: f64) -> (Vec<usize>, Vec<usize>) {
    let n = totals.len();
    let size = ((n as f64 * fraction) as usize).max(1).min(n / 2);
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| totals[a].total_cmp(&totals[b]));
    let lower = order[..size].to_vec();
    let upper = order[n - size..].to_vec();
    (lower, upper)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn central_values() {
        assert_eq!(mean(&[]), None);
        assert!(close(mean(&[1.0, 2.0, 6.0]).unwrap_or(0.0), 3.0));
        assert!(close(variance(&[1.0, 3.0]).unwrap_or(0.0), 1.0));
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), Some(2.5));
    }

    #[test]
    fn correlation_needs_variation() {
        assert_eq!(correlation(&[1.0, 1.0], &[1.0, 2.0]), None);
        let r = correlation(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]).unwrap_or(0.0);
        assert!(close(r, 1.0));
        let r = correlation(&[1.0, 2.0, 3.0], &[3.0, 2.0, 1.0]).unwrap_or(0.0);
        assert!(close(r, -1.0));
    }

    #[test]
    fn kr20_by_hand() {
        // 4 students, 3 items; totals 3, 2, 0, 0: mean 1.25, variance 1.6875.
        // Item p: 0.5, 0.5, 0.25, so pq sums to 0.25 + 0.25 + 0.1875 = 0.6875.
        // KR-20 = 3/2 * (1 - 0.6875 / 1.6875) = 0.8888...
        let items = vec![
            vec![1.0, 1.0, 1.0],
            vec![1.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0],
            vec![0.0, 0.0, 0.0],
        ];
        assert!(close(kr20(&items).unwrap_or(0.0), 8.0 / 9.0));
        assert_eq!(kr20(&[vec![1.0, 0.0]]), None, "one student");
        assert_eq!(
            kr20(&[vec![1.0, 1.0], vec![1.0, 1.0]]),
            None,
            "no variation"
        );
    }

    #[test]
    fn binomial_tail_by_hand() {
        // Six picks, all on one of three distractors: (1/3)^6.
        assert!(close(binomial_tail(6, 6, 1.0 / 3.0), 1.0 / 729.0));
        // Two fair coins, at least one head: 3/4.
        assert!(close(binomial_tail(2, 1, 0.5), 0.75));
        assert_eq!(binomial_tail(5, 0, 0.3), 1.0);
        assert_eq!(binomial_tail(5, 6, 0.3), 0.0);
    }

    #[test]
    fn groups_take_the_ends() {
        let totals = [5.0, 1.0, 3.0, 4.0, 2.0, 0.0, 6.0, 7.0, 8.0, 9.0];
        let (lower, upper) = extreme_groups(&totals, 0.27);
        // 10 students: groups of 2, as in the earlier tool.
        assert_eq!(lower, vec![5, 1]);
        assert_eq!(upper, vec![8, 9]);
        let (lower, upper) = extreme_groups(&[1.0, 2.0], 0.27);
        assert_eq!((lower, upper), (vec![0], vec![1]));
        assert_eq!(extreme_groups(&[], 0.27), (vec![], vec![]));
        assert_eq!(
            extreme_groups(&[3.0], 0.27),
            (vec![], vec![]),
            "one student: no groups"
        );
    }
}
