//! What the metered seats put in a request body, independent of any key.

/// The current models reject a fixed thinking budget with a 400, and the
/// older families have no adaptive mode — so the seat has to ask for the
/// one its model accepts.
#[test]
fn the_thinking_parameter_matches_what_the_model_accepts() {
    use mtg_player::llm::thinking_param;
    for current in ["claude-opus-5", "claude-opus-4-8", "claude-opus-4-7", "claude-sonnet-5", "claude-fable-5-1", "claude-sonnet-4-6"] {
        assert_eq!(thinking_param(current)["type"], "adaptive", "{current} takes adaptive thinking");
        assert!(thinking_param(current).get("budget_tokens").is_none(),
            "{current} rejects a fixed budget");
    }
    for older in ["claude-haiku-4-5", "claude-sonnet-4-5", "claude-3-5-haiku-20241022"] {
        assert_eq!(thinking_param(older)["type"], "enabled", "{older} has no adaptive mode");
        assert!(thinking_param(older)["budget_tokens"].as_u64().is_some_and(|b| b >= 1024),
            "{older} needs a budget of at least the API minimum");
    }
}
