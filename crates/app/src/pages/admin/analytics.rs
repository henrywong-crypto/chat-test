/// Admin analytics page — `/admin/analytics`.
/// Shows usage summary, model usage, and top-users tables.

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
            fallback=|| view! { <p>"Access denied — admin only."</p> }
        >
            {move || {
                analytics.get().map(|wrap| {
                    match (*wrap).clone() {
                        None    => view! { <p>"No analytics data available."</p> }.into_any(),
                        Some(d) => view! { <AnalyticsDashboard data=d /> }.into_any(),
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

    // Summary info table
    let summary = view! {
        <h2>"Summary"</h2>
        <table>
            <tr><td>"Conversations"</td><td>{data.total_conversations.to_string()}</td></tr>
            <tr><td>"Total tokens"</td><td>{format_number(total_tokens)}</td></tr>
            <tr><td>"Input / Output"</td>
                <td>{format_number(data.total_input_tokens)}" / "{format_number(data.total_output_tokens)}</td>
            </tr>
            <tr><td>"Estimated cost"</td><td>{format!("${:.4}", data.estimated_cost_usd)}</td></tr>
        </table>
    };

    // Model usage table
    let models = (!data.by_model.is_empty()).then(|| {
        let rows = data.by_model.clone().into_iter().map(|m| {
            let tokens = m.input_tokens + m.output_tokens;
            view! {
                <tr>
                    <td>{m.model_id}</td>
                    <td>{format_number(tokens)}</td>
                    <td>{format!("${:.4}", m.total_cost)}</td>
                </tr>
            }
        }).collect_view();
        view! {
            <h2>"Usage by Model"</h2>
            <table>
                <tr><th>"Model"</th><th>"Tokens"</th><th>"Cost"</th></tr>
                {rows}
            </table>
        }
    });

    // Top users table
    let top_users = (!data.top_users.is_empty()).then(|| {
        let rows = data.top_users.clone().into_iter().enumerate().map(|(i, u)| {
            view! {
                <tr>
                    <td>{(i + 1).to_string()}</td>
                    <td>{u.email}</td>
                    <td>{u.user_id}</td>
                    <td>{format_number(u.total_tokens)}</td>
                    <td>{format!("${:.4}", u.total_cost)}</td>
                </tr>
            }
        }).collect_view();
        view! {
            <h2>"Top Users"</h2>
            <table>
                <tr><th>"#"</th><th>"Email"</th><th>"User ID"</th><th>"Tokens"</th><th>"Cost"</th></tr>
                {rows}
            </table>
        }
    });

    view! {
        {summary}
        {models}
        {top_users}
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { out.push(','); }
        out.push(ch);
    }
    out.chars().rev().collect()
}
