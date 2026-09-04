//! What a run summary is allowed to claim a night of games cost. The seat
//! driven through the CLI spends plan quota and never bills tokens, and a
//! model with no published rate on file must say so rather than be priced
//! as if it were free or as if it were some other model.

use std::collections::HashMap;

use mtg_player::llm::{cost, is_plan_quota, model_prices, total_cost, Cost, LlmModelUsage};

fn usage(input: u64, output: u64, cache_read: u64, cache_create: u64) -> LlmModelUsage {
    LlmModelUsage { input, output, cache_read, cache_create, calls: 1 }
}

#[test]
fn a_plan_quota_seat_reports_no_dollar_figure_at_all() {
    let c = cost("claude-code", &usage(1_000_000, 1_000_000, 0, 0));
    assert_eq!(c, Cost::PlanQuota);
    assert_eq!(c.to_string(), "n/a (plan quota)");
}

#[test]
fn a_plan_quota_seat_with_a_model_suffix_is_still_plan_quota() {
    assert!(is_plan_quota("claude-code:opus"));
    assert_eq!(cost("claude-code:opus", &usage(500, 500, 0, 0)), Cost::PlanQuota);
}

#[test]
fn an_api_seat_is_priced_even_though_it_names_the_same_vendor() {
    assert!(!is_plan_quota("claude-sonnet-4-6"));
    assert!(matches!(cost("claude-sonnet-4-6", &usage(1, 0, 0, 0)), Cost::Usd(_)));
}

#[test]
fn a_metered_model_costs_its_published_rate_per_million_tokens() {
    // One million of each bucket makes the dollar figure the rate itself.
    let Cost::Usd(v) = cost("claude-sonnet-4-6", &usage(1_000_000, 1_000_000, 1_000_000, 1_000_000))
    else { panic!("a metered model must produce a dollar figure") };
    let p = model_prices("claude-sonnet-4-6").unwrap();
    assert!((v - (p.input + p.output + p.cache_read + p.cache_write)).abs() < 1e-9);
}

#[test]
fn a_model_with_no_rate_on_file_reports_unknown_rather_than_zero() {
    let c = cost("some-model-nobody-here-has-priced", &usage(1_000_000, 1_000_000, 0, 0));
    assert_eq!(c, Cost::Unknown);
    assert_eq!(c.to_string(), "unknown");
}

#[test]
fn the_models_the_api_seats_default_to_have_rates_on_file() {
    for model in ["claude-sonnet-4-6", "gemini-2.5-flash"] {
        assert!(model_prices(model).is_some(), "{model} has no rate on file");
    }
}

#[test]
fn a_run_of_only_plan_quota_seats_has_no_total_to_report() {
    let mut map = HashMap::new();
    map.insert("claude-code".to_string(), usage(10_000, 2_000, 0, 0));
    map.insert("claude-code:opus".to_string(), usage(10_000, 2_000, 0, 0));
    assert_eq!(total_cost(&map), Cost::PlanQuota);
}

#[test]
fn a_plan_quota_seat_adds_nothing_to_a_mixed_total() {
    let mut metered = HashMap::new();
    metered.insert("claude-sonnet-4-6".to_string(), usage(30_000, 4_000, 0, 0));
    let mut mixed = metered.clone();
    mixed.insert("claude-code".to_string(), usage(900_000, 90_000, 0, 0));
    assert_eq!(total_cost(&mixed), total_cost(&metered));
}

#[test]
fn one_unpriced_model_makes_the_whole_total_unknown() {
    let mut map = HashMap::new();
    map.insert("claude-sonnet-4-6".to_string(), usage(30_000, 4_000, 0, 0));
    map.insert("some-model-nobody-here-has-priced".to_string(), usage(30_000, 4_000, 0, 0));
    assert_eq!(total_cost(&map), Cost::Unknown);
}
