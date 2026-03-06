/// Per-model pricing and cost calculation.
///
/// All prices are in USD per 1 000 tokens.

use shared::TokenUsage;

use crate::models::get_model;

// ── Regional multipliers ──────────────────────────────────────────────────────

/// Some AWS regions apply a surcharge (e.g. ap-southeast-2 is 1.2× list price
/// for some Claude models).  Returns the multiplier for the given region.
fn regional_multiplier(region: &str) -> f64 {
    match region {
        // US regions — baseline pricing
        "us-east-1" | "us-east-2" | "us-west-2" => 1.0,
        // EU regions — typically +20 %
        "eu-west-1" | "eu-west-3" | "eu-central-1" => 1.2,
        // AP regions — typically +20–30 %
        "ap-northeast-1" | "ap-southeast-1" | "ap-southeast-2" => 1.2,
        "ap-south-1" => 1.25,
        // Default: no surcharge
        _ => 1.0,
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Calculate the USD cost for a single model response.
///
/// Returns `0.0` for unknown model IDs (fails silently — cost data is
/// best-effort and should never block a request).
pub fn calculate_message_cost(model_id: &str, usage: &TokenUsage, region: &str) -> f64 {
    let Some(model) = get_model(model_id) else {
        return 0.0;
    };

    let p   = &model.pricing;
    let mul = regional_multiplier(region);

    let input_cost  = (usage.input_tokens  as f64 / 1_000.0) * p.input_per_1k  * mul;
    let output_cost = (usage.output_tokens as f64 / 1_000.0) * p.output_per_1k * mul;
    let cw_cost     = (usage.cache_write_tokens as f64 / 1_000.0) * p.cache_write_per_1k * mul;
    let cr_cost     = (usage.cache_read_tokens  as f64 / 1_000.0) * p.cache_read_per_1k  * mul;

    input_cost + output_cost + cw_cost + cr_cost
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use shared::TokenUsage;

    #[test]
    fn cost_claude_3_5_sonnet_us() {
        // 1 000 input + 500 output, no cache
        let usage = TokenUsage { input_tokens: 1_000, output_tokens: 500, ..Default::default() };
        let cost = calculate_message_cost("claude-3-5-sonnet-v2", &usage, "us-east-1");
        // $3 / 1M in → $0.003, $15 / 1M out × 0.5k → $0.0075
        let expected = 0.003 + 0.0075;
        assert!((cost - expected).abs() < 1e-9, "cost={cost} expected={expected}");
    }

    #[test]
    fn cost_nova_micro_us() {
        let usage = TokenUsage { input_tokens: 10_000, output_tokens: 2_000, ..Default::default() };
        let cost = calculate_message_cost("nova-micro", &usage, "us-east-1");
        let expected = 10.0 * 0.000035 + 2.0 * 0.00014;
        assert!((cost - expected).abs() < 1e-9, "cost={cost} expected={expected}");
    }

    #[test]
    fn cost_unknown_model_returns_zero() {
        let usage = TokenUsage { input_tokens: 1_000, output_tokens: 1_000, ..Default::default() };
        assert_eq!(calculate_message_cost("does-not-exist", &usage, "us-east-1"), 0.0);
    }

    #[test]
    fn regional_multiplier_eu() {
        let usage = TokenUsage { input_tokens: 1_000, output_tokens: 0, ..Default::default() };
        let us = calculate_message_cost("claude-3-5-sonnet-v2", &usage, "us-east-1");
        let eu = calculate_message_cost("claude-3-5-sonnet-v2", &usage, "eu-west-1");
        assert!((eu - us * 1.2).abs() < 1e-10, "eu={eu} us*1.2={}", us * 1.2);
    }
}
