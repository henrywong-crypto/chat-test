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
        Either::Left(view! { <a href={href}>"← Previous"</a> })
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
        Either::Left(view! { <a href={href}>"Next →"</a> })
    } else {
        Either::Right(())
    };

    view! {
        <p class="pagination-nav">{info}" "{prev}" "{next}</p>
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
            <div class="hub-page">
                // ── Breadcrumb h1 ─────────────────────────────────────────────
                {if !breadcrumbs.is_empty() {
                    Either::Left(view! {
                        <h1>
                            {breadcrumbs.into_iter().enumerate().map(|(index, crumb)| {
                                let sep = if index > 0 { " / " } else { "" };
                                match crumb.href {
                                    Some(href) => Either::Left(view! {
                                        {sep}<a href={href}>{crumb.label}</a>
                                    }),
                                    None => Either::Right(view! {
                                        {sep}{crumb.label}
                                    }),
                                }
                            }).collect::<Vec<_>>()}
                        </h1>
                    })
                } else {
                    Either::Right(view! { <h1>{title}</h1> })
                }}

                // ── Navigation ────────────────────────────────────────────────
                {if !nav_links.is_empty() {
                    Either::Left(view! {
                        <h2>"Navigation"</h2>
                        <table>
                            {nav_links.into_iter().map(|link| view! {
                                <tr><td><a href={link.href}>{link.label}</a></td></tr>
                            }).collect::<Vec<_>>()}
                        </table>
                    })
                } else {
                    Either::Right(())
                }}

                // ── Info ──────────────────────────────────────────────────────
                {if !info_rows.is_empty() {
                    Either::Left(view! {
                        <h2>"Info"</h2>
                        <table>
                            {info_rows.into_iter().map(|row| view! {
                                <tr><td>{row.label}</td><td>{row.value}</td></tr>
                            }).collect::<Vec<_>>()}
                        </table>
                    })
                } else {
                    Either::Right(())
                }}

                // ── Content ───────────────────────────────────────────────────
                {content}

                // ── Subpages ──────────────────────────────────────────────────
                {if !subpages.is_empty() {
                    Either::Left(view! {
                        <h2>"Subpages"</h2>
                        <table>
                            <tr><th>"Page"</th><th>"Count"</th></tr>
                            {subpages.into_iter().map(|sub| view! {
                                <tr>
                                    <td><a href={sub.href}>{sub.label}</a></td>
                                    <td>{sub.count}</td>
                                </tr>
                            }).collect::<Vec<_>>()}
                        </table>
                    })
                } else {
                    Either::Right(())
                }}
            </div>
        }
    }
}
