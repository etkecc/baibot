use std::time::Duration;

/// After this much elapsed generation time, the notice escalates to the "medium" pool.
const TIER_MEDIUM_AFTER: Duration = Duration::from_secs(30);

/// After this much elapsed generation time, the notice escalates to the "deep" pool.
const TIER_DEEP_AFTER: Duration = Duration::from_secs(90);

/// Early flavor: the response is just taking a moment longer than instant.
const MESSAGES_LIGHT: &[&str] = &[
    "*{{ baibot_name }} is thinking…*",
    "*{{ baibot_name }} is mulling that over…*",
    "*Hmm, let me think about that…*",
    "*{{ baibot_name }} is gathering some thoughts…*",
    "*One moment, {{ baibot_name }} is working on it…*",
    "*{{ baibot_name }} is putting the pieces together…*",
    "*Give {{ baibot_name }} a second here…*",
    "*Let me think this one through…*",
    "*{{ baibot_name }} is warming up the gears…*",
    "*{{ baibot_name }} is on it…*",
];

/// Mid flavor: this is a real question and the model is genuinely working.
const MESSAGES_MEDIUM: &[&str] = &[
    "*Huh, good one. {{ baibot_name }} is really thinking now…*",
    "*Still working on it, {{ baibot_name }} wants to get this right…*",
    "*This one needs a bit more thought…*",
    "*{{ baibot_name }} is digging into this…*",
    "*Hang tight, {{ baibot_name }} is turning it over…*",
    "*Not a quick one, this. {{ baibot_name }} is still at it…*",
    "*{{ baibot_name }} is chewing on this properly now…*",
    "*This deserves some real thought, bear with {{ baibot_name }}…*",
    "*{{ baibot_name }} is working through the details…*",
    "*Won't be long, {{ baibot_name }} is closing in on it…*",
];

/// Deep flavor: a long-running generation (e.g. a reasoning model going for minutes).
const MESSAGES_DEEP: &[&str] = &[
    "*Okay, this is a hard one. {{ baibot_name }} is really deep in thought…*",
    "*{{ baibot_name }} is in the weeds on this one, thanks for your patience…*",
    "*Still here, still thinking. {{ baibot_name }} doesn't want to rush it…*",
    "*A proper puzzle, this. {{ baibot_name }} is taking the time to do it justice…*",
    "*{{ baibot_name }} is really wrestling with this one…*",
    "*A meaty question. {{ baibot_name }} is still turning it over…*",
    "*{{ baibot_name }} hasn't forgotten you, just thinking hard…*",
    "*Almost there, {{ baibot_name }} is pulling it all together…*",
    "*{{ baibot_name }} is going the extra mile on this one…*",
    "*Deep thoughts in progress. {{ baibot_name }} appreciates your patience…*",
];

/// Returns the flavor pool matching how long generation has been running.
pub fn messages_for_elapsed(elapsed: Duration) -> &'static [&'static str] {
    if elapsed >= TIER_DEEP_AFTER {
        MESSAGES_DEEP
    } else if elapsed >= TIER_MEDIUM_AFTER {
        MESSAGES_MEDIUM
    } else {
        MESSAGES_LIGHT
    }
}

/// Picks one (still-untemplated) flavor message from the tier active at `elapsed`,
/// indexed by a monotonic `sequence` the caller increments once per notice.
///
/// Using a monotonic counter (rather than a clock-derived value) guarantees two
/// things the gesture-novelty goal needs: consecutive notices never repeat a line
/// (the index advances by one each tick, so it differs whenever the tier has more
/// than one message), and the pool is fully walked before any line recurs. The
/// caller seeds `sequence` with a per-generation value so different turns don't all
/// open on the same line. A clock-derived index can't promise this: `interval_at`
/// ticks on a near-fixed schedule, so its sub-second component clusters and would
/// re-pick the same line.
pub fn pick_message(elapsed: Duration, sequence: usize) -> &'static str {
    let pool = messages_for_elapsed(elapsed);
    pool[sequence % pool.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_for_elapsed_returns_the_right_tier() {
        // Boundaries: light below 30s, medium [30s, 90s), deep at/after 90s.
        // The pools have distinct content, so value comparison identifies the tier.
        assert_eq!(messages_for_elapsed(Duration::from_secs(0)), MESSAGES_LIGHT);
        assert_eq!(
            messages_for_elapsed(Duration::from_secs(29)),
            MESSAGES_LIGHT
        );
        assert_eq!(
            messages_for_elapsed(Duration::from_secs(30)),
            MESSAGES_MEDIUM
        );
        assert_eq!(
            messages_for_elapsed(Duration::from_secs(89)),
            MESSAGES_MEDIUM
        );
        assert_eq!(messages_for_elapsed(Duration::from_secs(90)), MESSAGES_DEEP);
        assert_eq!(
            messages_for_elapsed(Duration::from_secs(600)),
            MESSAGES_DEEP
        );
    }

    #[test]
    fn every_tier_is_non_empty() {
        for pool in [MESSAGES_LIGHT, MESSAGES_MEDIUM, MESSAGES_DEEP] {
            assert!(!pool.is_empty());
            for message in pool {
                assert!(!message.trim().is_empty());
            }
        }
    }

    #[test]
    fn every_tier_uses_the_bot_name_template() {
        // Not every line names the bot (some are first-person flavor), but each
        // tier exercises the template var so substitution is wired through.
        for pool in [MESSAGES_LIGHT, MESSAGES_MEDIUM, MESSAGES_DEEP] {
            assert!(pool.iter().any(|m| m.contains("{{ baibot_name }}")));
        }
    }

    #[test]
    fn pick_message_stays_within_the_active_tier() {
        let deep = messages_for_elapsed(Duration::from_secs(120));
        for sequence in 0..50 {
            assert!(deep.contains(&pick_message(Duration::from_secs(120), sequence)));
        }
    }

    #[test]
    fn pick_message_never_repeats_on_consecutive_sequences() {
        // The gesture-novelty guarantee: a monotonic sequence must not pick the same line twice
        // in a row (and walks the whole tier before any line recurs).
        let elapsed = Duration::from_secs(0);
        let pool_len = messages_for_elapsed(elapsed).len();
        for sequence in 0..(pool_len * 3) {
            assert_ne!(
                pick_message(elapsed, sequence),
                pick_message(elapsed, sequence + 1),
            );
        }
    }
}
