"""Test sending a chat message via the UI with Playwright."""
from playwright.sync_api import sync_playwright

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page()

    console_msgs = []
    page.on("console", lambda msg: console_msgs.append(f"[{msg.type}] {msg.text}"))

    errors = []
    page.on("pageerror", lambda err: errors.append(str(err)))

    # Track network requests/responses for /api/chat
    chat_responses = []
    def handle_response(response):
        if "/api/chat" in response.url:
            chat_responses.append({
                "status": response.status,
                "url": response.url,
            })
    page.on("response", handle_response)

    print("--- Loading page ---")
    page.goto("http://localhost:3000/", wait_until="networkidle", timeout=30000)

    # Wait for hydration
    page.wait_for_timeout(2000)

    # Type a message
    print("--- Typing 'hi' ---")
    textarea = page.query_selector(".message-input")
    textarea.fill("hi")

    # Click send
    print("--- Clicking Send ---")
    page.click(".btn-send")

    # Wait for response (up to 15 seconds)
    print("--- Waiting for response ---")
    page.wait_for_timeout(10000)

    # Take screenshot
    page.screenshot(path="/home/ubuntu/chat-test/screenshot_chat.png", full_page=True)
    print("--- Screenshot saved to screenshot_chat.png ---")

    # Check for assistant message
    assistant_msgs = page.query_selector_all(".assistant-message, .message-assistant, .msg-assistant")
    print(f"--- Assistant messages found: {len(assistant_msgs)} ---")
    for msg in assistant_msgs:
        text = msg.inner_text()[:200]
        print(f"  > {text}")

    # Check for any error toasts or error displays
    toasts = page.query_selector_all(".toast, .toast-error, .error")
    print(f"--- Toasts/errors found: {len(toasts)} ---")
    for t in toasts:
        print(f"  > {t.inner_text()[:200]}")

    # Check message list area
    msg_list = page.query_selector(".message-list, .messages, .chat-messages")
    if msg_list:
        print(f"--- Message list content ---")
        print(msg_list.inner_text()[:500])
    else:
        # Try to find any new content in the chat area
        chat = page.query_selector(".chat-page")
        if chat:
            print(f"--- Chat page content ---")
            print(chat.inner_text()[:500])

    # Check chat API responses
    print(f"\n--- Chat API responses: {len(chat_responses)} ---")
    for r in chat_responses:
        print(f"  {r}")

    # Console messages
    print(f"\n--- Console messages ({len(console_msgs)}) ---")
    for msg in console_msgs:
        if "error" in msg.lower() or "warn" in msg.lower() or "bedrock" in msg.lower() or "chat" in msg.lower():
            print(f"  {msg}")

    print(f"\n--- Page errors ({len(errors)}) ---")
    for err in errors:
        print(f"  {err}")

    browser.close()
    print("\n--- Done ---")
