//! What a drafting seat's answer to a pick amounts to.

/// What a seat's answer amounted to: the card it picked, and whether that
/// card was actually chosen or substituted because the answer was unusable.
///
/// The substitution itself is deliberate — a draft has to continue — but it
/// used to be silent, so 42 unparsable answers produced 42 confident
/// "Chose:" lines and a tournament built on them (issue #195). The adjacent
/// backend code already treats this class of failure as loud; this carries
/// the same fact out of the parser so the caller can too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pick {
    /// The seat named this card.
    Chosen(String),
    /// The answer could not be used; this is the first card of the pack.
    Substituted(String),
}

impl Pick {
    #[must_use]
    pub fn card(&self) -> &str {
        match self {
            Pick::Chosen(c) | Pick::Substituted(c) => c,
        }
    }

    #[must_use]
    pub fn into_card(self) -> String {
        match self {
            Pick::Chosen(c) | Pick::Substituted(c) => c,
        }
    }

    #[must_use]
    pub fn was_substituted(&self) -> bool {
        matches!(self, Pick::Substituted(_))
    }
}

/// Read a seat's pick answer against the pack it was asked about.
///
/// # Panics
/// `available` is empty: a seat is never asked to pick from nothing.
#[must_use]
pub fn parse_pick_response(response: &str, available: &[String]) -> Pick {
    // Primary path: JSON response like `{"thoughts": "...", "pick": N}`.
    // Secondary path (legacy or stray wrappers): strip markdown code fences
    // and retry. Last resort: fall through to a text scan for "PICK: N".
    let try_json = |s: &str| -> Option<String> {
        let v: serde_json::Value = serde_json::from_str(s).ok()?;
        let idx = usize::try_from(v["pick"].as_u64()?).unwrap_or(usize::MAX);
        (idx < available.len()).then(|| available[idx].clone())
    };

    if let Some(pick) = try_json(response) {
        return Pick::Chosen(pick);
    }

    // Strip optional ```json ... ``` fencing that some models still add.
    let stripped = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let Some(pick) = try_json(stripped) {
        return Pick::Chosen(pick);
    }

    // Legacy text scan — kept for robustness against older responses.
    for line in response.lines().rev() {
        let trimmed = line.trim().to_uppercase();
        if let Some(rest) = trimmed.strip_prefix("PICK:") {
            if let Ok(idx) = rest.trim().trim_start_matches('"').trim_end_matches('"').trim_end_matches(',').parse::<usize>() {
                if idx < available.len() {
                    return Pick::Chosen(available[idx].clone());
                }
            }
        }
    }

    // Last resort: the draft must continue, so take the first card — but say
    // so, rather than letting it pass for a decision.
    Pick::Substituted(available[0].clone())
}

#[cfg(test)]
mod pick_parsing_tests {
    use super::{parse_pick_response, Pick};

    fn pack() -> Vec<String> {
        ["Hysterical Blindness", "Voiceless Spirit", "Ambush Viper", "Delver of Secrets"]
            .iter().map(std::string::ToString::to_string).collect()
    }

    #[test]
    fn a_usable_answer_is_the_seats_own_pick() {
        let p = pack();
        assert_eq!(parse_pick_response(r#"{"pick": 2}"#, &p),
            Pick::Chosen("Ambush Viper".into()));
        assert_eq!(parse_pick_response("```json\n{\"pick\": 1}\n```", &p),
            Pick::Chosen("Voiceless Spirit".into()));
        assert_eq!(parse_pick_response("thinking...\nPICK: 3", &p),
            Pick::Chosen("Delver of Secrets".into()));
    }

    /// The four shapes from issue #195: each is a well-formed JSON object
    /// that never reaches the backend's loud "no structured object" path,
    /// so the parser is the only place that can notice. Each still yields a
    /// card — a draft has to continue — but it must be marked as the
    /// runner's substitution, not the seat's choice.
    #[test]
    fn an_unusable_answer_is_reported_as_a_substitution() {
        let p = pack();
        for response in [
            r#"{"pick": 9999}"#,            // out-of-range index
            r#"{"choice": 3}"#,             // right shape, wrong key
            "{}",                           // empty object
            r#"{"pick": "Ambush Viper"}"#,  // a name where an index goes
        ] {
            let got = parse_pick_response(response, &p);
            assert_eq!(got, Pick::Substituted("Hysterical Blindness".into()),
                "{response} is not a usable pick, so it must not pass for one");
            assert!(got.was_substituted(),
                "{response} must be reportable as a substitution");
            // The draft still gets a card to continue with.
            assert_eq!(got.card(), "Hysterical Blindness");
        }
    }
}
