"""Comprehensive Playwright audit of all pages."""
from playwright.sync_api import sync_playwright
import os

PAGES = [
    ("/", "home"),
    ("/conversations", "conversations"),
    ("/bots", "my_bots"),
    ("/bots/store", "bot_store"),
    ("/c/new", "chat_new"),
    ("/admin/users", "admin_users"),
    ("/admin/analytics", "admin_analytics"),
]

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page()

    for path, name in PAGES:
        url = f"http://localhost:3000{path}"
        print(f"\n{'='*60}")
        print(f"PAGE: {name} ({url})")
        print('='*60)

        console_msgs = []
        errors = []
        page.on("console", lambda msg: console_msgs.append(f"[{msg.type}] {msg.text}"))
        page.on("pageerror", lambda err: errors.append(str(err)))

        try:
            page.goto(url, wait_until="networkidle", timeout=15000)
        except Exception as e:
            print(f"  LOAD ERROR: {e}")
            continue

        page.wait_for_timeout(2000)

        # Screenshot
        ss_path = f"/home/ubuntu/chat-test/ss_{name}.png"
        page.screenshot(path=ss_path, full_page=True)
        print(f"  Screenshot: {ss_path}")

        # Title
        print(f"  Title: {page.title()}")

        # Body text preview
        body = page.inner_text("body")[:300]
        print(f"  Body preview: {body[:200]}")

        # Check for hydration errors
        hydration_errors = [m for m in console_msgs if "hydration" in m.lower()]
        if hydration_errors:
            print(f"  HYDRATION ERRORS: {len(hydration_errors)}")
            for e in hydration_errors:
                print(f"    {e[:200]}")

        # Page errors
        if errors:
            print(f"  PAGE ERRORS: {len(errors)}")
            for e in errors:
                print(f"    {e[:150]}")

        # Check CSS loaded
        css_link = page.query_selector('link[rel="stylesheet"]')
        if css_link:
            href = css_link.get_attribute("href")
            print(f"  CSS href: {href}")

        # Check visible elements
        h1 = page.query_selector("h1")
        if h1:
            print(f"  H1: {h1.inner_text()[:100]}")

        tables = page.query_selector_all("table")
        print(f"  Tables: {len(tables)}")

        # Reset listeners for next page
        console_msgs.clear()
        errors.clear()

    browser.close()
    print("\n\nDone.")
