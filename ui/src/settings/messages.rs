use leptos::prelude::*;
use serde::Serialize;
use tospu_model::{MessageDraft, MessageItem, MessageKind, MAX_AUTHOR_CHARS, MAX_TEXT_CHARS};
use wasm_bindgen::prelude::*;

use crate::ipc;
use crate::settings::store::{EmptyArgs, Store};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Kind(MessageKind),
}

#[derive(Serialize)]
struct AddArgs {
    draft: MessageDraft,
}

#[derive(Serialize)]
struct UpdateArgs {
    item: MessageItem,
}

#[derive(Serialize)]
struct DeleteArgs {
    id: String,
}

fn kind_label(kind: MessageKind) -> &'static str {
    match kind {
        MessageKind::Goal => "Goal",
        MessageKind::Quote => "Quote",
        MessageKind::Gratitude => "Gratitude",
        MessageKind::Proud => "Pride",
    }
}

fn kind_value(kind: MessageKind) -> &'static str {
    match kind {
        MessageKind::Goal => "goal",
        MessageKind::Quote => "quote",
        MessageKind::Gratitude => "gratitude",
        MessageKind::Proud => "proud",
    }
}

fn parse_kind(s: &str) -> MessageKind {
    match s {
        "goal" => MessageKind::Goal,
        "quote" => MessageKind::Quote,
        "gratitude" => MessageKind::Gratitude,
        "proud" => MessageKind::Proud,
        _ => MessageKind::Goal,
    }
}

fn empty_copy(filter: Filter) -> &'static str {
    match filter {
        Filter::All => "No messages yet. Add a goal, quote, gratitude or pride note above.",
        Filter::Kind(MessageKind::Goal) => {
            "No goals yet. What's one big thing you're working toward?"
        }
        Filter::Kind(MessageKind::Quote) => "No quotes yet. Add a line you want to come back to.",
        Filter::Kind(MessageKind::Gratitude) => {
            "Nothing here yet. What's one thing you're grateful for today?"
        }
        Filter::Kind(MessageKind::Proud) => "Nothing here yet. What's something you're proud of?",
    }
}

#[component]
pub fn MessagesTab() -> impl IntoView {
    let store = use_context::<Store>().expect("Store provided");

    let filter = RwSignal::new(Filter::All);
    let draft_kind = RwSignal::new(MessageKind::Goal);
    let draft_text = RwSignal::new(String::new());
    let draft_author = RwSignal::new(String::new());

    let editing_id = RwSignal::new(None::<String>);
    let edit_kind = RwSignal::new(MessageKind::Goal);
    let edit_text = RwSignal::new(String::new());
    let edit_author = RwSignal::new(String::new());
    let edit_enabled = RwSignal::new(true);

    // Undo toast state: (deleted item, sequence counter so older timers don't clear newer toasts)
    let undo_item = RwSignal::new(None::<(MessageItem, u64)>);
    let undo_seq = RwSignal::new(0u64);

    let set_filter = move |f: Filter| {
        filter.set(f);
        if let Filter::Kind(k) = f {
            draft_kind.set(k);
        }
    };

    let submit_add = move || {
        let kind = draft_kind.get_untracked();
        let text = draft_text.get_untracked();
        let author = if kind == MessageKind::Quote {
            Some(draft_author.get_untracked())
        } else {
            None
        };
        let draft = MessageDraft { kind, text, author };
        store.run_then(
            async move { ipc::invoke::<_, MessageItem>("add_message", &AddArgs { draft }).await },
            move |_| {
                draft_text.set(String::new());
                draft_author.set(String::new());
            },
        );
    };

    let trigger_undo_toast = move |removed: MessageItem| {
        let next_seq = undo_seq.get_untracked() + 1;
        undo_seq.set(next_seq);
        undo_item.set(Some((removed, next_seq)));

        if let Some(win) = web_sys::window() {
            let cb = Closure::once_into_js(move || {
                if undo_seq.get_untracked() == next_seq {
                    undo_item.set(None);
                }
            });
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                6000,
            );
        }
    };

    let writable = move || {
        store
            .view
            .get()
            .map(|v| v.content_writable)
            .unwrap_or(false)
    };

    let content_problem = move || {
        store.view.get().and_then(|v| {
            v.problems
                .into_iter()
                .find(|p| p.starts_with("content.json"))
        })
    };

    let privacy_blur = move || {
        store
            .view
            .get()
            .map(|v| v.settings.privacy_blur)
            .unwrap_or(true)
    };

    let filtered_messages = move || {
        let f = filter.get();
        store
            .view
            .get()
            .map(|v| {
                v.messages
                    .into_iter()
                    .filter(|m| match f {
                        Filter::All => true,
                        Filter::Kind(k) => m.kind == k,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };

    let char_count = move || draft_text.get().chars().count();

    view! {
        <section class="tab-panel" aria-label="Messages">
            <header class="panel-header">
                <h2>"Messages"</h2>
            </header>

            {move || {
                if !writable() {
                    let problem = content_problem()
                        .unwrap_or_else(|| "content.json could not be read.".to_string());
                    Some(view! {
                        <div class="alert-box">
                            <p>{problem}</p>
                            <button
                                type="button"
                                class="btn secondary"
                                on:click=move |_| {
                                    store.run_then(
                                        async {
                                            ipc::invoke::<_, tospu_model::SettingsView>(
                                                "reload_files",
                                                &EmptyArgs {},
                                            )
                                            .await
                                        },
                                        move |fresh| store.view.set(Some(fresh)),
                                    );
                                }
                            >
                                "Reload files"
                            </button>
                        </div>
                    })
                } else {
                    None
                }
            }}

            <div class="chips" role="group" aria-label="Filter messages">
                <button
                    type="button"
                    class=move || if filter.get() == Filter::All { "chip active" } else { "chip" }
                    on:click=move |_| set_filter(Filter::All)
                >
                    "All"
                </button>
                <button
                    type="button"
                    class=move || {
                        if filter.get() == Filter::Kind(MessageKind::Goal) {
                            "chip active"
                        } else {
                            "chip"
                        }
                    }
                    on:click=move |_| set_filter(Filter::Kind(MessageKind::Goal))
                >
                    "Goals"
                </button>
                <button
                    type="button"
                    class=move || {
                        if filter.get() == Filter::Kind(MessageKind::Quote) {
                            "chip active"
                        } else {
                            "chip"
                        }
                    }
                    on:click=move |_| set_filter(Filter::Kind(MessageKind::Quote))
                >
                    "Quotes"
                </button>
                <button
                    type="button"
                    class=move || {
                        if filter.get() == Filter::Kind(MessageKind::Gratitude) {
                            "chip active"
                        } else {
                            "chip"
                        }
                    }
                    on:click=move |_| set_filter(Filter::Kind(MessageKind::Gratitude))
                >
                    "Gratitude"
                </button>
                <button
                    type="button"
                    class=move || {
                        if filter.get() == Filter::Kind(MessageKind::Proud) {
                            "chip active"
                        } else {
                            "chip"
                        }
                    }
                    on:click=move |_| set_filter(Filter::Kind(MessageKind::Proud))
                >
                    "Pride"
                </button>
            </div>

            <form
                class="add-form"
                on:submit=move |ev| {
                    ev.prevent_default();
                    submit_add();
                }
            >
                <div class="add-row">
                    <select
                        aria-label="Message kind"
                        disabled=move || !writable()
                        prop:value=move || kind_value(draft_kind.get())
                        on:change=move |ev| draft_kind.set(parse_kind(&event_target_value(&ev)))
                    >
                        <option value="goal">"Goal"</option>
                        <option value="quote">"Quote"</option>
                        <option value="gratitude">"Gratitude"</option>
                        <option value="proud">"Pride"</option>
                    </select>
                    <span class="char-counter">
                        {move || format!("{}/{}", char_count(), MAX_TEXT_CHARS)}
                    </span>
                </div>

                <textarea
                    rows="2"
                    maxlength=MAX_TEXT_CHARS
                    placeholder="Write a message..."
                    disabled=move || !writable()
                    prop:value=move || draft_text.get()
                    on:input=move |ev| draft_text.set(event_target_value(&ev))
                    on:keydown=move |ev: web_sys::KeyboardEvent| {
                        if (ev.meta_key() || ev.ctrl_key()) && ev.key() == "Enter" {
                            ev.prevent_default();
                            submit_add();
                        }
                    }
                ></textarea>

                {move || {
                    if draft_kind.get() == MessageKind::Quote {
                        Some(view! {
                            <input
                                type="text"
                                maxlength=MAX_AUTHOR_CHARS
                                placeholder="Author (optional)"
                                disabled=move || !writable()
                                prop:value=move || draft_author.get()
                                on:input=move |ev| draft_author.set(event_target_value(&ev))
                            />
                        })
                    } else {
                        None
                    }
                }}

                <div class="add-actions">
                    <button
                        type="submit"
                        class="btn primary"
                        disabled=move || !writable() || draft_text.get().trim().is_empty()
                    >
                        "Add"
                    </button>
                </div>
            </form>

            <div class="message-list">
                {move || {
                    let list = filtered_messages();
                    if list.is_empty() {
                        view! { <p class="empty-state">{empty_copy(filter.get())}</p> }.into_any()
                    } else {
                        view! {
                            <For
                                each=move || filtered_messages()
                                key=|item| {
                                    (
                                        item.id.clone(),
                                        item.kind,
                                        item.text.clone(),
                                        item.author.clone(),
                                        item.enabled,
                                    )
                                }
                                children=move |item| {
                                    let item_id = item.id.clone();
                                    let item_id_check = item.id.clone();
                                    let item_for_toggle = item.clone();
                                    let item_for_edit = item.clone();
                                    let item_for_delete = item.id.clone();
                                    let is_editing = move || {
                                        editing_id.get().as_deref() == Some(item_id_check.as_str())
                                    };

                                    view! {
                                        <div class=move || {
                                            if item.enabled {
                                                "message-row"
                                            } else {
                                                "message-row disabled-row"
                                            }
                                        }>
                                            {move || {
                                                if is_editing() {
                                                    let save_id = item_id.clone();
                                                    view! {
                                                        <div class="edit-inline">
                                                            <select
                                                                aria-label="Edit kind"
                                                                prop:value=move || kind_value(edit_kind.get())
                                                                on:change=move |ev| {
                                                                    edit_kind.set(parse_kind(&event_target_value(&ev)))
                                                                }
                                                            >
                                                                <option value="goal">"Goal"</option>
                                                                <option value="quote">"Quote"</option>
                                                                <option value="gratitude">"Gratitude"</option>
                                                                <option value="proud">"Pride"</option>
                                                            </select>
                                                            <textarea
                                                                rows="2"
                                                                maxlength=MAX_TEXT_CHARS
                                                                prop:value=move || edit_text.get()
                                                                on:input=move |ev| edit_text.set(event_target_value(&ev))
                                                            ></textarea>
                                                            {move || {
                                                                if edit_kind.get() == MessageKind::Quote {
                                                                    Some(view! {
                                                                        <input
                                                                            type="text"
                                                                            maxlength=MAX_AUTHOR_CHARS
                                                                            placeholder="Author (optional)"
                                                                            prop:value=move || edit_author.get()
                                                                            on:input=move |ev| {
                                                                                edit_author.set(event_target_value(&ev))
                                                                            }
                                                                        />
                                                                    })
                                                                } else {
                                                                    None
                                                                }
                                                            }}
                                                            <div class="row-actions">
                                                                <button
                                                                    type="button"
                                                                    class="btn primary small"
                                                                    on:click=move |_| {
                                                                        let k = edit_kind.get_untracked();
                                                                        let updated = MessageItem {
                                                                            id: save_id.clone(),
                                                                            kind: k,
                                                                            text: edit_text.get_untracked(),
                                                                            author: if k == MessageKind::Quote {
                                                                                Some(edit_author.get_untracked())
                                                                            } else {
                                                                                None
                                                                            },
                                                                            enabled: edit_enabled.get_untracked(),
                                                                        };
                                                                        store.run_then(
                                                                            async move {
                                                                                ipc::invoke::<_, MessageItem>(
                                                                                    "update_message",
                                                                                    &UpdateArgs { item: updated },
                                                                                )
                                                                                .await
                                                                            },
                                                                            move |_| editing_id.set(None),
                                                                        );
                                                                    }
                                                                >
                                                                    "Save"
                                                                </button>
                                                                <button
                                                                    type="button"
                                                                    class="btn secondary small"
                                                                    on:click=move |_| editing_id.set(None)
                                                                >
                                                                    "Cancel"
                                                                </button>
                                                            </div>
                                                        </div>
                                                    }
                                                    .into_any()
                                                } else {
                                                    let toggle_item = item_for_toggle.clone();
                                                    let edit_item = item_for_edit.clone();
                                                    let del_id = item_for_delete.clone();
                                                    let display_text = item.text.clone();
                                                    let display_author = item.author.clone();
                                                    let badge = kind_label(item.kind);
                                                    let checked = item.enabled;

                                                    view! {
                                                        <label class="switch" title="Enable or disable">
                                                            <input
                                                                type="checkbox"
                                                                disabled=move || !writable()
                                                                prop:checked=checked
                                                                on:change=move |_| {
                                                                    let mut next = toggle_item.clone();
                                                                    next.enabled = !next.enabled;
                                                                    store.run(async move {
                                                                        ipc::invoke::<_, MessageItem>(
                                                                            "update_message",
                                                                            &UpdateArgs { item: next },
                                                                        )
                                                                        .await
                                                                    });
                                                                }
                                                            />
                                                            <span class="slider"></span>
                                                        </label>

                                                        <div
                                                            class=move || {
                                                                if privacy_blur() {
                                                                    "message-body blur"
                                                                } else {
                                                                    "message-body"
                                                                }
                                                            }
                                                            tabindex="0"
                                                        >
                                                            <span class="kind-badge">{badge}</span>
                                                            <span class="message-text">{display_text}</span>
                                                            {display_author.map(|a| {
                                                                view! { <span class="message-author">{format!("— {a}")}</span> }
                                                            })}
                                                        </div>

                                                        <div class="row-actions">
                                                            <button
                                                                type="button"
                                                                class="btn ghost small"
                                                                disabled=move || !writable()
                                                                on:click=move |_| {
                                                                    editing_id.set(Some(edit_item.id.clone()));
                                                                    edit_kind.set(edit_item.kind);
                                                                    edit_text.set(edit_item.text.clone());
                                                                    edit_author.set(
                                                                        edit_item.author.clone().unwrap_or_default(),
                                                                    );
                                                                    edit_enabled.set(edit_item.enabled);
                                                                }
                                                            >
                                                                "Edit"
                                                            </button>
                                                            <button
                                                                type="button"
                                                                class="btn ghost danger small"
                                                                disabled=move || !writable()
                                                                on:click=move |_| {
                                                                    let id = del_id.clone();
                                                                    store.run_then(
                                                                        async move {
                                                                            ipc::invoke::<_, MessageItem>(
                                                                                "delete_message",
                                                                                &DeleteArgs { id },
                                                                            )
                                                                            .await
                                                                        },
                                                                        trigger_undo_toast,
                                                                    );
                                                                }
                                                            >
                                                                "Delete"
                                                            </button>
                                                        </div>
                                                    }
                                                    .into_any()
                                                }
                                            }}
                                        </div>
                                    }
                                }
                            />
                        }
                        .into_any()
                    }
                }}
            </div>

            {move || {
                undo_item.get().map(|(removed, _)| {
                    view! {
                        <div class="toast" role="status">
                            <span>"Deleted"</span>
                            <span aria-hidden="true">" · "</span>
                            <button
                                type="button"
                                class="toast-undo"
                                on:click=move |_| {
                                    let draft = MessageDraft {
                                        kind: removed.kind,
                                        text: removed.text.clone(),
                                        author: removed.author.clone(),
                                    };
                                    undo_item.set(None);
                                    store.run(async move {
                                        ipc::invoke::<_, MessageItem>(
                                            "add_message",
                                            &AddArgs { draft },
                                        )
                                        .await
                                    });
                                }
                            >
                                "Undo"
                            </button>
                        </div>
                    }
                })
            }}
        </section>
    }
}
