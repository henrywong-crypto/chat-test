/// Admin analytics page — `/admin/analytics`.
/// Shows usage summary cards, a CSS bar chart by model, and a top-users table.

use leptos::prelude::*;
use shared::UsageAnalyticsResponse;
use templates::{Breadcrumb, Page};

use crate::context::auth::use_auth;

// ── AdminAnalyticsPage ────────────────────────────────────────────────────────

#[component]
pub fn AdminAnalyticsPage() -> impl IntoView {
    let auth = use_auth();

    let analytics = LocalResource::new(move || {
        let token = auth.get().map(|u| u.token).unwrap_or_default();
        async move {
            if token.is_empty() { return None::<UsageAnalyticsResponse>; }
            #[cfg(feature = "hydrate")]
            { crate::api::fetch_analytics(&token).await.ok() }
            #[cfg(not(feature = "hydrate"))]
            { None }
        }
    });

    let is_admin = move || auth.get().map(|u| u.is_admin).unwrap_or(false);

    let dashboard = view! {
        <Show
            when=is_admin
            fallback=|| view! { <p class="admin-denied">"Access denied — admin only."</p> }
        >
            {move || {
                analytics.get().map(|wrap| {
                    match (*wrap).clone() {
                        None => view! {
                            <p class="text-muted">"No analytics data available."</p>
                        }.into_any(),
                        Some(data) => view! { <AnalyticsDashboard data=data /> }.into_any(),
                    }
                })
            }}
        </Show>
    };

    Page {
        title: "Usage Analytics".to_string(),
        breadcrumbs: vec![Breadcrumb::current("Usage Analytics")],
        nav_links: vec![],
        info_rows: vec![],
        content: dashboard,
        subpages: vec![],
    }
    .into_view()
}

// ── AnalyticsDashboard ────────────────────────────────────────────────────────

#[component]
fn AnalyticsDashboard(data: UsageAnalyticsResponse) -> impl IntoView {
    let total_tokens = data.total_input_tokens + data.total_output_tokens;

    // CSS bar chart: find max tokens across all models for scaling.
    let max_tokens = data.by_model.iter()
        .map(|m| m.input_tokens + m.output_tokens)
        .max()
        .unwrap_or(1)
        .max(1);

    view! {
        // ── Summary cards ─────────────────────────────────────────────────────
        <div class="stat-cards">
            <div class="stat-card">
                <div class="stat-label">"Total Conversations"</div>
                <div class="stat-value">{data.total_conversations.to_string()}</div>
            </div>
            <div class="stat-card">
                <div class="stat-label">"Total Tokens"</div>
                <div class="stat-value">{format_number(total_tokens)}</div>
            </div>
            <div class="stat-card">
                <div class="stat-label">"Estimated Cost"</div>
                <div class="stat-value">{format!("${:.4}", data.estimated_cost_usd)}</div>
            </div>
            <div class="stat-card">
                <div class="stat-label">"Input / Output"</div>
                <div class="stat-value">
                    {format_number(data.total_input_tokens)}
                    " / "
                    {format_number(data.total_output_tokens)}
                </div>
            </div>
        </div>

        // ── Model usage bar chart ─────────────────────────────────────────────
        {(!data.by_model.is_empty()).then(|| {
            let bars = data.by_model.clone().into_iter().map(|m| {
                let tokens = m.input_tokens + m.output_tokens;
                let pct    = (tokens as f64 / max_tokens as f64 * 100.0) as u32;
                view! {
                    <div class="bar-row">
                        <div class="bar-label truncate">{m.model_id.clone()}</div>
                        <div class="bar-track">
                            <div class="bar-fill" style=format!("width:{}%", pct)></div>
                        </div>
                        <div class="bar-tokens">{format_number(tokens)}</div>
                        <div class="bar-cost">{format!("${:.4}", m.total_cost)}</div>
                    </div>
                }
            }).collect_view();

            view! {
                <section class="analytics-section">
                    <h2 class="analytics-section-title">"Usage by Model"</h2>
                    <div class="bar-chart">{bars}</div>
                </section>
            }
        })}

        // ── Top users table ───────────────────────────────────────────────────
        {(!data.top_users.is_empty()).then(|| {
            let rows = data.top_users.clone().into_iter().enumerate().map(|(i, u)| {
                view! {
                    <tr>
                        <td>{(i + 1).to_string()}</td>
                        <td>{u.email}</td>
                        <td class="mono">{u.user_id}</td>
                        <td class="number">{format_number(u.total_tokens)}</td>
                        <td class="number">{format!("${:.4}", u.total_cost)}</td>
                    </tr>
                }
            }).collect_view();

            view! {
                <section class="analytics-section">
                    <h2 class="analytics-section-title">"Top Users"</h2>
                    <table class="admin-table">
                        <thead>
                            <tr>
                                <th>"#"</th>
                                <th>"Email"</th>
                                <th>"User ID"</th>
                                <th>"Tokens"</th>
                                <th>"Cost"</th>
                            </tr>
                        </thead>
                        <tbody>{rows}</tbody>
                    </table>
                </section>
            }
        })}
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn format_number(n: u64) -> String {
    // Simple thousands separator.
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { out.push(','); }
        out.push(ch);
    }
    out.chars().rev().collect()
}
