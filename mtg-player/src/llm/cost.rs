//! What a seat's recorded token usage actually cost.
//!
//! Two of the seats speak the same prompt protocol to the same vendor but
//! are paid for in different currencies: the Messages API seat bills tokens
//! to an API key, while the seat driven through the CLI spends the CLI
//! login's plan quota and is never billed per token. A run summary that
//! prints dollars for the second one is telling the reader they spent money
//! they did not spend, and that number is exactly what a user reads to
//! decide whether a night of games was affordable — so the two are reported
//! apart, and a model with no rate on file says so instead of being priced
//! at zero (or, worse, quietly priced as if it were some other model).

use std::collections::HashMap;
use std::fmt;

use super::LlmModelUsage;

/// Dollars per million tokens for one model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPrices {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

/// What a run spent on one model — or, for a plan-quota seat, the fact that
/// the question does not apply.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cost {
    /// Spent against a plan quota; no API money changed hands.
    PlanQuota,
    /// Metered tokens at a rate this build has on file.
    Usd(f64),
    /// Metered tokens whose rate this build does not know.
    Unknown,
}

impl fmt::Display for Cost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cost::PlanQuota => f.write_str("n/a (plan quota)"),
            Cost::Usd(v) => write!(f, "${v:.4}"),
            Cost::Unknown => f.write_str("unknown"),
        }
    }
}

/// Whether usage recorded under this label was paid for out of a plan quota
/// rather than metered API tokens. The label is the one the CLI-driven
/// backend records under, bare or with a `:model` suffix.
#[must_use]
pub fn is_plan_quota(model: &str) -> bool {
    model == "claude-code" || model.starts_with("claude-code:")
}

/// Published rates ($/MTok) for the models the seats can request, or `None`
/// when this build has no rate on file for the name.
///
/// Anthropic: platform.claude.com/docs/en/about-claude/pricing (verified 2026-04-08)
/// Gemini: ai.google.dev/pricing (verified 2026-04-08)
#[must_use]
pub fn model_prices(model: &str) -> Option<ModelPrices> {
    let p = |input, output, cache_read, cache_write| {
        Some(ModelPrices { input, output, cache_read, cache_write })
    };
    match model {
        // Anthropic (cache read is 0.1x input, cache write 1.25x input at the 5-minute TTL).
        m if m.contains("fable-5") || m.contains("mythos-5") => p(10.00, 50.00, 1.00, 12.50),
        m if m.contains("opus-5") || m.contains("opus-4-8") || m.contains("opus-4-7") => p(5.00, 25.00, 0.50, 6.25),
        m if m.contains("sonnet-5") => p(2.00, 10.00, 0.20, 2.50),
        m if m.contains("opus-4-6") || m.contains("opus-4-5") => p(5.00, 25.00, 0.50, 6.25),
        m if m.contains("opus-4-1") => p(15.00, 75.00, 1.50, 18.75),
        m if m.contains("sonnet-4-6") || m.contains("sonnet-4-5") => p(3.00, 15.00, 0.30, 3.75),
        m if m.contains("sonnet-4-0") || m.contains("sonnet-4-2") => p(3.00, 15.00, 0.30, 3.75),
        m if m.contains("haiku-4-5") => p(1.00, 5.00, 0.10, 1.25),
        m if m.contains("haiku-3-5") => p(0.80, 4.00, 0.08, 1.00),
        // Gemini (cache read is 0.1x input; implicit caching has no write charge).
        m if m.contains("gemini-2.5-flash-lite") => p(0.10, 0.40, 0.01, 0.0),
        m if m.contains("gemini-2.5-flash") => p(0.30, 2.50, 0.03, 0.0),
        m if m.contains("gemini-2.5-pro") => p(1.25, 10.00, 0.125, 0.0),
        m if m.contains("gemini-3.1-flash-lite") => p(0.25, 1.50, 0.025, 0.0),
        m if m.contains("gemini-3.1-pro") => p(2.00, 12.00, 0.20, 0.0),
        m if m.contains("gemini-3-flash") || m.contains("gemini-3.0-flash") => p(0.50, 3.00, 0.05, 0.0),
        m if m.contains("gemini-3-pro") || m.contains("gemini-3.0-pro") => p(2.00, 12.00, 0.20, 0.0),
        // A name nobody here has a rate for. Guessing a neighbouring
        // model's price reads as a real figure and is off by up to 5x, so
        // the caller is told the rate is missing instead.
        _ => None,
    }
}

/// The cost of one model's recorded usage.
#[must_use]
pub fn cost(model: &str, usage: &LlmModelUsage) -> Cost {
    if is_plan_quota(model) {
        return Cost::PlanQuota;
    }
    let Some(prices) = model_prices(model) else {
        return Cost::Unknown;
    };
    // Token counts are converted through u32 because realistic counts fit
    // and saturating there keeps a corrupt total from becoming a wild price.
    let tok = |n: u64| f64::from(u32::try_from(n).unwrap_or(u32::MAX));
    Cost::Usd(
        tok(usage.input) * prices.input / 1_000_000.0
            + tok(usage.output) * prices.output / 1_000_000.0
            + tok(usage.cache_read) * prices.cache_read / 1_000_000.0
            + tok(usage.cache_create) * prices.cache_write / 1_000_000.0,
    )
}

/// The cost of a whole run's usage map. A run of nothing but plan-quota
/// seats has no dollar total to report, and one priced model with no rate
/// on file makes the total unknown rather than an understatement.
#[must_use]
pub fn total_cost(usage: &HashMap<String, LlmModelUsage>) -> Cost {
    let mut metered = None;
    for (model, stats) in usage {
        match cost(model, stats) {
            Cost::PlanQuota => {}
            Cost::Unknown => return Cost::Unknown,
            Cost::Usd(v) => *metered.get_or_insert(0.0) += v,
        }
    }
    match metered {
        Some(v) => Cost::Usd(v),
        None if usage.is_empty() => Cost::Usd(0.0),
        None => Cost::PlanQuota,
    }
}
