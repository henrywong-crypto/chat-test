"""Quick Playwright smoke-test for the Leptos UI."""
from playwright.sync_api import sync_playwright
import json, sys

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page()

    # Collect console messages and errors
    console_msgs = []
    page.on("console", lambda msg: console_msgs.append(f"[{msg.type}] {msg.text}"))

    errors = []
    page.on("pageerror", lambda err: errors.append(str(err)))

    print("--- Loading http://localhost:3000/ ---")
    page.goto("http://localhost:3000/", wait_until="networkidle", timeout=30000)

    print(f"\n--- Page title: {page.title()} ---")

    # Check if WASM loaded
    print(f"\n--- URL: {page.url} ---")

    # Screenshot
    page.screenshot(path="/home/ubuntu/chat-test/screenshot.png", full_page=True)
    print("--- Screenshot saved to screenshot.png ---")

    # Check key elements
    print("\n--- Element checks ---")
    checks = {
        "sidebar":       ".sidebar",
        "user-name":     ".user-name",
        "chat-page":     ".chat-page",
        "message-input": ".message-input",
        "model-selector": ".model-selector",
        "nav-item":      ".nav-item",
    }
    for name, sel in checks.items():
        el = page.query_selector(sel)
        if el:
            text = el.inner_text()[:80].strip()
            print(f"  {name}: FOUND - '{text}'")
        else:
            print(f"  {name}: MISSING")

    # Check user-name specifically
    user_name = page.query_selector(".user-name")
    if user_name:
        print(f"\n--- User display: '{user_name.inner_text()}' ---")

    # Check for admin section (should appear with dev bypass)
    admin = page.query_selector(".sidebar-section-label")
    print(f"--- Admin section visible: {admin is not None} ---")

    # Wait a bit more for hydration effects
    page.wait_for_timeout(3000)

    # Re-check user after hydration
    user_name2 = page.query_selector(".user-name")
    if user_name2:
        print(f"--- User after hydration wait: '{user_name2.inner_text()}' ---")

    # Check console
    print(f"\n--- Console messages ({len(console_msgs)}) ---")
    for msg in console_msgs:
        print(f"  {msg}")

    print(f"\n--- Page errors ({len(errors)}) ---")
    for err in errors:
        print(f"  {err}")

    # Try clicking send without message
    send_btn = page.query_selector(".btn-send")
    if send_btn:
        print(f"\n--- Send button enabled: {send_btn.is_enabled()} ---")

    browser.close()
    print("\n--- Done ---")
