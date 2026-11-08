import numpy as np
import pytest
import scipy.stats as stats
from python.research.validate_gates import (
    calc_deflated_sharpe,
    stationary_bootstrap,
)


def test_stationary_bootstrap_constant():
    # If all returns are constant +1.0R, all bounds must be identically +1.0R
    pnl = np.ones(50)
    lcb, median, ucb = stationary_bootstrap(pnl, n_resamples=500, mean_block_len=3.0)
    assert pytest.approx(lcb) == 1.0
    assert pytest.approx(median) == 1.0
    assert pytest.approx(ucb) == 1.0


def test_stationary_bootstrap_ordering():
    rng = np.random.default_rng(123)
    pnl = rng.normal(loc=0.20, scale=1.0, size=100)
    lcb, median, ucb = stationary_bootstrap(pnl, n_resamples=1000, mean_block_len=3.0)
    assert lcb < median < ucb
    assert lcb > -0.10  # true mean 0.20 with SE 0.10


def test_calc_deflated_sharpe_properties():
    # 1. When SR_hat == SR*, Z == 0 and DSR == 0.50
    sr_star, z, dsr = calc_deflated_sharpe(
        sr_hat=0.15,
        n_samples=100,
        skew=0.0,
        kurt_pearson=3.0,
        k_trials=5,
        var_trials=0.01,
    )
    # If sr_hat is exactly sr_star, z = 0, dsr = 0.5
    sr_star_fixed = sr_star
    _, z_eq, dsr_eq = calc_deflated_sharpe(
        sr_hat=sr_star_fixed,
        n_samples=100,
        skew=0.0,
        kurt_pearson=3.0,
        k_trials=5,
        var_trials=0.01,
    )
    assert pytest.approx(z_eq, abs=1e-5) == 0.0
    assert pytest.approx(dsr_eq, abs=1e-5) == 0.50

    # 2. When SR_hat >> SR*, DSR approaches 1.0
    _, z_high, dsr_high = calc_deflated_sharpe(
        sr_hat=0.80,
        n_samples=100,
        skew=0.0,
        kurt_pearson=3.0,
        k_trials=5,
        var_trials=0.01,
    )
    assert z_high > 3.0
    assert dsr_high > 0.99
