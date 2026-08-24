//! Ported dari packages/opencode/src/session/overflow.ts dan
//! session/compaction.ts (subset: isOverflow, usable, pruning constants).

const COMPACTION_BUFFER: u64 = 20_000;

/// Minimal model limit info untuk compaction (subset dari oc_provider::Model).
pub struct ModelLimits {
    pub context: u64,
    pub input: Option<u64>,
    pub output: u64,
}

/// Compaction pruning thresholds (compaction.ts:28-29).
pub const PRUNE_MINIMUM: u64 = 20_000;
pub const PRUNE_PROTECT: u64 = 40_000;

/// Input untuk `usable` / `isOverflow`.
pub struct OverflowInput<'a> {
    /// compaction.reserved dari config (None = default)
    pub reserved: Option<u64>,
    /// compaction.auto (None = true)
    pub auto: Option<bool>,
    pub limits: &'a ModelLimits,
    pub output_token_max: Option<usize>,
}

/// Ported from: overflow.ts `usable()`
pub fn usable(input: &OverflowInput) -> u64 {
    let context = input.limits.context;
    if context == 0 {
        return 0;
    }

    let max_output = input.output_token_max.unwrap_or(32_000) as u64;
    let reserved = input
        .reserved
        .unwrap_or_else(|| COMPACTION_BUFFER.min(max_output));

    match input.limits.input {
        Some(input_limit) => input_limit.saturating_sub(reserved),
        None => context.saturating_sub(max_output.min(input.limits.output).max(1)),
    }
}

/// Token counts untuk overflow check.
#[derive(Debug, Clone, Copy, Default)]
pub struct TokenCounts {
    pub total: Option<u64>,
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

/// Ported from: overflow.ts `isOverflow()`
pub fn is_overflow(input: &OverflowInput, tokens: &TokenCounts) -> bool {
    if input.auto == Some(false) {
        return false;
    }
    if input.limits.context == 0 {
        return false;
    }

    let count = tokens
        .total
        .unwrap_or_else(|| tokens.input + tokens.output + tokens.cache_read + tokens.cache_write);
    count >= usable(input)
}

/// Tool output pruning: bila token usage > PRUNE_PROTECT, tool outputs lama
/// yang melebihi PRUNE_MINIMUM dipangkas.
/// Mengembalikan daftar index pesan yang perlu di-prune.
pub fn should_prune(tokens: &TokenCounts) -> bool {
    let count = tokens
        .total
        .unwrap_or_else(|| tokens.input + tokens.output + tokens.cache_read + tokens.cache_write);
    count > PRUNE_PROTECT
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_limits(context: u64, input: Option<u64>, output: u64) -> ModelLimits {
        ModelLimits {
            context,
            input,
            output,
        }
    }

    #[test]
    fn usable_zero_context_returns_zero() {
        let limits = make_limits(0, None, 8192);
        let input = OverflowInput {
            reserved: None,
            auto: None,
            limits: &limits,
            output_token_max: None,
        };
        assert_eq!(usable(&input), 0);
    }

    #[test]
    fn usable_with_input_limit() {
        let limits = make_limits(200_000, Some(100_000), 8192);
        let input = OverflowInput {
            reserved: None,
            auto: None,
            limits: &limits,
            output_token_max: None,
        };
        assert_eq!(usable(&input), 80_000);
    }

    #[test]
    fn usable_without_input_limit_uses_context_minus_output() {
        let limits = make_limits(200_000, None, 8192);
        let input = OverflowInput {
            reserved: None,
            auto: None,
            limits: &limits,
            output_token_max: None,
        };
        assert_eq!(usable(&input), 191_808); // min(32k, 8192)=8192 → 200k-8192
    }

    #[test]
    fn overflow_when_tokens_exceed_usable() {
        let limits = make_limits(200_000, Some(100_000), 8192); // usable≈80k
        let input = OverflowInput {
            reserved: None,
            auto: None,
            limits: &limits,
            output_token_max: None,
        };

        let tokens = TokenCounts {
            input: 85_000,
            ..Default::default()
        };
        assert!(is_overflow(&input, &tokens));

        let tokens = TokenCounts {
            input: 70_000,
            ..Default::default()
        };
        assert!(!is_overflow(&input, &tokens));
    }

    #[test]
    fn overflow_disabled_when_auto_false() {
        let limits = make_limits(200_000, Some(100_000), 8192);
        let input = OverflowInput {
            reserved: None,
            auto: Some(false),
            limits: &limits,
            output_token_max: None,
        };

        let tokens = TokenCounts {
            input: 999_999,
            ..Default::default()
        };
        assert!(!is_overflow(&input, &tokens));
    }

    #[test]
    fn overflow_total_takes_precedence_over_sum() {
        let limits = make_limits(200_000, Some(100_000), 8192);
        let input = OverflowInput {
            reserved: None,
            auto: None,
            limits: &limits,
            output_token_max: None,
        };

        let tokens = TokenCounts {
            total: Some(90_000),
            input: 10_000,
            ..Default::default()
        };
        assert!(is_overflow(&input, &tokens));
    }

    #[test]
    fn prune_threshold_check() {
        assert!(!should_prune(&TokenCounts {
            input: 39_999,
            ..Default::default()
        }));
        assert!(should_prune(&TokenCounts {
            input: 40_001,
            ..Default::default()
        }));
    }
}
