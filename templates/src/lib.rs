use leptos::{either::Either, prelude::*};

// ── Breadcrumb ────────────────────────────────────────────────────────────────

pub struct Breadcrumb {
    pub label: String,
    pub href: Option<String>,
}

impl Breadcrumb {
    pub fn link(label: impl ToString, href: impl ToString) -> Self {
        Self { label: label.to_string(), href: Some(href.to_string()) }
    }

    pub fn current(label: impl ToString) -> Self {
        Self { label: label.to_string(), href: None }
    }
}

// ── NavLink ───────────────────────────────────────────────────────────────────

pub struct NavLink {
    pub label: String,
    pub href: String,
}

impl NavLink {
    pub fn new(label: impl ToString, href: impl ToString) -> Self {
        Self { label: label.to_string(), href: href.to_string() }
    }

    pub fn back() -> Self {
        Self { label: "Back".to_string(), href: "javascript:history.back()".to_string() }
    }
}

// ── InfoRow ───────────────────────────────────────────────────────────────────

pub struct InfoRow {
    pub label: String,
    pub value: AnyView,
}

impl InfoRow {
    pub fn new(label: &str, value: &str) -> Self {
        let value_string = value.to_string();
        Self { label: label.to_string(), value: value_string.into_any() }
    }

    pub fn view(label: &str, value: impl IntoView + 'static) -> Self {
        Self { label: label.to_string(), value: value.into_any() }
    }
}

// ── Subpage ───────────────────────────────────────────────────────────────────

pub struct Subpage {
    pub label: String,
    pub href: String,
    pub count: String,
}

impl Subpage {
    pub fn new(label: impl ToString, href: impl ToString, count: impl std::fmt::Display) -> Self {
        Self {
            label: label.to_string(),
            href: href.to_string(),
            count: count.to_string(),
        }
    }
}

// ── Pagination ────────────────────────────────────────────────────────────────

pub struct Pagination {
    pub current_page: i64,
    pub total_pages: i64,
    pub total_items: i64,
    pub base_url: String,
    pub extra_params: String,
}

impl Pagination {
    pub fn new(
        current_page: i64,
        total_items: i64,
        per_page: i64,
        base_url: impl ToString,
        extra_params: impl ToString,
    ) -> Self {
        let total_pages = if total_items == 0 {
            1
        } else {
            (total_items + per_page - 1) / per_page
        };
        Self {
            current_page,
            total_pages,
            total_items,
            base_url: base_url.to_string(),
            extra_params: extra_params.to_string(),
        }
    }
}

pub fn pagination_nav(pagination: &Pagination) -> AnyView {
    if pagination.total_pages <= 1 {
        return ().into_any();
    }

    let info = format!("Page {} of {}", pagination.current_page, pagination.total_pages);
    let prev = if pagination.current_page > 1 {
        let href = format!(
            "{}?page={}{}",
            pagination.base_url,
            pagination.current_page - 1,
            pagination.extra_params
        );
        Either::Left(view! { <a href={href} class="btn btn-secondary btn-sm">"← Previous"</a> })
    } else {
        Either::Right(())
    };
    let next = if pagination.current_page < pagination.total_pages {
        let href = format!(
            "{}?page={}{}",
            pagination.base_url,
            pagination.current_page + 1,
            pagination.extra_params
        );
        Either::Left(view! { <a href={href} class="btn btn-secondary btn-sm">"Next →"</a> })
    } else {
        Either::Right(())
    };

    view! {
        <div class="pagination-nav">
            <span class="pagination-info">{info}</span>
            {prev}
            {next}
        </div>
    }
    .into_any()
}

// ── Page ──────────────────────────────────────────────────────────────────────

pub struct Page<C: IntoView = ()> {
    pub title: String,
    pub breadcrumbs: Vec<Breadcrumb>,
    pub nav_links: Vec<NavLink>,
    pub info_rows: Vec<InfoRow>,
    pub content: C,
    pub subpages: Vec<Subpage>,
}

impl Default for Page {
    fn default() -> Self {
        Page {
            title: String::new(),
            breadcrumbs: Vec::new(),
            nav_links: Vec::new(),
            info_rows: Vec::new(),
            content: (),
            subpages: Vec::new(),
        }
    }
}

impl<C: IntoView + 'static> Page<C> {
    pub fn into_view(self) -> impl IntoView {
        let Page { title, breadcrumbs, nav_links, info_rows, content, subpages } = self;

        view! {
            <div class="page-content">
                // ── Header ────────────────────────────────────────────────────
                <div class="page-header">
                    {if breadcrumbs.is_empty() {
                        Either::Left(view! {
                            <h1 class="page-title">{title}</h1>
                        })
                    } else {
                        Either::Right(view! {
                            <h1 class="page-title page-breadcrumb">
                                {breadcrumbs.into_iter().enumerate().map(|(index, crumb)| {
                                    let sep = if index > 0 {
                                        Either::Left(view! { <span class="breadcrumb-sep">" / "</span> })
                                    } else {
                                        Either::Right(())
                                    };
                                    match crumb.href {
                                        Some(href) => Either::Left(view! {
                                            {sep}<a href={href} class="breadcrumb-link">{crumb.label}</a>
                                        }),
                                        None => Either::Right(view! {
                                            {sep}<span class="breadcrumb-current">{crumb.label}</span>
                                        }),
                                    }
                                }).collect::<Vec<_>>()}
                            </h1>
                        })
                    }}

                    // Nav links rendered as action buttons in the header
                    {if !nav_links.is_empty() {
                        Either::Left(view! {
                            <div class="page-actions">
                                {nav_links.into_iter().map(|link| view! {
                                    <a href={link.href} class="btn btn-secondary btn-sm">{link.label}</a>
                                }).collect::<Vec<_>>()}
                            </div>
                        })
                    } else {
                        Either::Right(())
                    }}
                </div>

                // ── Info rows ─────────────────────────────────────────────────
                {if !info_rows.is_empty() {
                    Either::Left(view! {
                        <table class="info-table">
                            {info_rows.into_iter().map(|row| view! {
                                <tr>
                                    <th>{row.label}</th>
                                    <td>{row.value}</td>
                                </tr>
                            }).collect::<Vec<_>>()}
                        </table>
                    })
                } else {
                    Either::Right(())
                }}

                // ── Main content ──────────────────────────────────────────────
                {content}

                // ── Subpages ──────────────────────────────────────────────────
                {if !subpages.is_empty() {
                    Either::Left(view! {
                        <div class="subpages-grid">
                            {subpages.into_iter().map(|sub| view! {
                                <a href={sub.href} class="subpage-card">
                                    <span class="subpage-label">{sub.label}</span>
                                    <span class="subpage-count">{sub.count}</span>
                                </a>
                            }).collect::<Vec<_>>()}
                        </div>
                    })
                } else {
                    Either::Right(())
                }}
            </div>
        }
    }
}
