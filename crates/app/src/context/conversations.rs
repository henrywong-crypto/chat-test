/// Conversation list context — lets ChatPage signal the Sidebar to reload.

use leptos::prelude::*;

/// A simple version counter.  Incrementing it causes the Sidebar's
/// conversation `LocalResource` to re-fetch.
#[derive(Clone, Copy)]
pub struct ConversationContext {
    pub version: RwSignal<u32>,
}

pub fn provide_conversation_context() {
    provide_context(ConversationContext {
        version: RwSignal::new(0),
    });
}

pub fn use_conversation_context() -> ConversationContext {
    expect_context()
}
