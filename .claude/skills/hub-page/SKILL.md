---
name: hub-page
description: HTML/CSS conventions for bedrock-rs pages. Use when writing or modifying pages in crates/app/src/pages/. Based on gateway-proxy-rs html-style conventions.
user-invocable: false
---

# Hub page conventions

## Framework

Pages use **`templates::Page`** for standard sections and **Leptos 0.7** (`view!` macro) for custom content. Every page function is a `#[component]` that returns `impl IntoView`. Call `Page { ... }.into_view()` to produce the view.

**No modern UI components. No sidebar. No navbar. No modal.** Plain monospace HTML, same aesthetic as gateway-proxy-rs.

```rust
use leptos::prelude::*;
use templates::{Breadcrumb, NavLink, Page};

#[component]
pub fn ExamplePage() -> impl IntoView {
    let content = view! {
        <h2>"Items"</h2>
        <p>"Custom content here"</p>
    };

    Page {
        title:       "Example".to_string(),
        breadcrumbs: vec![
            Breadcrumb::link("Bedrock RS", "/"),
            Breadcrumb::current("Example"),
        ],
        nav_links:   vec![NavLink::back()],
        info_rows:   vec![],
        content,
        subpages:    vec![],
    }
    .into_view()
}
```

When `content` is `()`, use `..Default::default()` for the remaining fields. When content is a view, **all fields must be listed explicitly**.

### `Page` struct

| Field | Type | Purpose |
|-------|------|---------|
| `title` | `String` | Page heading (used as `<h1>` when no breadcrumbs) |
| `breadcrumbs` | `Vec<Breadcrumb>` | `<h1>` breadcrumb trail |
| `nav_links` | `Vec<NavLink>` | Navigation section links |
| `info_rows` | `Vec<InfoRow>` | Key-value info table |
| `content` | `C: IntoView` | Custom page content (Leptos view or `()`) |
| `subpages` | `Vec<Subpage>` | Subpages table with Page/Count columns |

Renders in order: breadcrumbs/title, nav_links, info_rows, content, subpages. Empty sections are omitted.

### All helpers

| Helper | Purpose |
|--------|---------|
| `Breadcrumb::link(label, href)` | Linked breadcrumb |
| `Breadcrumb::current(label)` | Terminal breadcrumb (plain text) |
| `NavLink::new(label, href)` | Navigation link |
| `NavLink::back()` | "Back" via `javascript:history.back()` — always last |
| `InfoRow::new(label, value)` | Plain text value |
| `InfoRow::view(label, view)` | Leptos view value |
| `Subpage::new(label, href, count)` | Subpage entry — count accepts any `Display` |
| `Pagination::new(page, total_items, per_page, base_url, extra_params)` | Builds pagination state |
| `pagination_nav(&pagination)` | Previous/Next links. Returns `AnyView` (empty when single page) |

## Leptos `view!` syntax

- Text nodes: `"Click me"`
- Interpolation: `{variable}` or `{expression}`
- Conditionals: `Either::Left(view! { ... })` / `Either::Right(())` to hide
- Optional: `Some(view! { ... })` / `None`
- Iteration: `.into_iter().map(|item| view! { ... }).collect_view()`
- Type-erased: `.into_any()` returns `AnyView`
- Reactive closure as view: `move || { ... .into_any() }` (re-runs when tracked signals change)

## CSS

Hub pages use the `.hub-page` class in `style/main.scss`. The class applies:

```css
.hub-page {
  font-family: monospace;
  padding: 16px;
}
```

Table, `th`, `td`, `h1`, `h2`, `a`, `pre` inside `.hub-page` are styled automatically.
**Do not add Tailwind classes or inline styles inside hub-page content.** Let default browser styles + `.hub-page` do the work.

## Routes

```
/                  # Home — subpages list
/conversations     # All conversations (list table)
/c/new             # New conversation (chat)
/c/:id             # Existing conversation (chat)
/bots              # My bots
/bots/store        # Bot store
/bots/new          # Create bot form
/bots/:id/edit     # Edit bot form
/admin/users       # Admin: user list
/admin/analytics   # Admin: usage analytics
```

## Page types

| Page type | Key fields |
|-----------|------------|
| **Home** | no breadcrumbs, subpages with links |
| **List** | breadcrumbs, nav_links ("New …"), content = table with rows |
| **Detail / Chat** | breadcrumbs (reactive title), nav_links, content = messages + form |
| **Edit form** | breadcrumbs, nav_links (Back), content = `<table>` form |

### Breadcrumbs

All ancestors are links; terminal is `Breadcrumb::current`. Home page: no breadcrumbs (use `title` field only).

### Title format

`"Bedrock RS"` for home, plain noun otherwise: `"Conversations"`, `"My Bots"`, `"Create Bot"`, `"Edit Bot"`.

## Tables

### List table

```rust
view! {
    <table>
        <tr><th>"Title"</th><th>"ID"</th><th></th></tr>
        {items.into_iter().map(|item| {
            let href = format!("/c/{}", item.id);
            view! {
                <tr>
                    <td><a href={href}>{item.title}</a></td>
                    <td>{item.id}</td>
                    <td>
                        <a href={edit_href}>"Edit"</a>
                        " "
                        <a href="#" on:click=...>"Delete"</a>
                    </td>
                </tr>
            }
        }).collect_view()}
    </table>
}
```

Action column: last `<th>` is empty, actions separated by `" "`.

## Forms

```rust
view! {
    <form on:submit=on_submit>
        <table>
            <tr>
                <td><label for="name">"Name"</label></td>
                <td><input id="name" type="text" required=true size="60"
                    prop:value=value
                    on:input=move |ev| set_value.set(event_target_value(&ev))
                /></td>
            </tr>
            <tr>
                <td></td>
                <td>
                    <button type="submit" prop:disabled=submitting>"Save"</button>
                    " "
                    <a href="/back">"Cancel"</a>
                </td>
            </tr>
        </table>
    </form>
}
```

- Text inputs: `size="60"`. Textareas: `rows="6"`.
- Submit labels: "Create" (new), "Save changes" (edit), "Send" (chat).
- Error messages: plain `<p>{error}</p>` above the form.

## Async data loading

Use `LocalResource` for client-side data fetching. Guard API calls with `#[cfg(feature = "hydrate")]`:

```rust
let data = LocalResource::new(move || {
    let token = auth.get().map(|u| u.token).unwrap_or_default();
    async move {
        if token.is_empty() { return vec![]; }
        #[cfg(feature = "hydrate")]
        { crate::api::fetch_something(&token).await.unwrap_or_default() }
        #[cfg(not(feature = "hydrate"))]
        { vec![] }
    }
});
```

In views: `data.get().map(|wrap| { let list = (*wrap).clone(); ... })`.

## Feature guards

Variables only used in `#[cfg(feature = "hydrate")]` blocks must be suppressed in SSR builds:

```rust
#[cfg(not(feature = "hydrate"))]
let _ = (&navigate, &conv_ctx);
```
