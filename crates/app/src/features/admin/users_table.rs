use leptos::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::i18n::*;

use crate::features::admin::api::{
    admin_users_create, admin_users_delete, admin_users_search, admin_users_update,
    admin_users_update_with_password,
};
use crate::features::admin::types::AdminUserRow;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum PendingKind {
    Save,
    Delete,
}

type RowHandle = (i64, ArcRwSignal<AdminUserRow>);

fn normalize_roles(mut roles: Vec<String>) -> Vec<String> {
    let allowed = ["user", "admin", "staff"];
    let mut out = roles
        .drain(..)
        .map(|r| r.trim().to_lowercase())
        .filter(|r| allowed.contains(&r.as_str()))
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

fn roles_to_csv(roles: &[String]) -> String {
    normalize_roles(roles.to_vec()).join(",")
}

fn toggle_role(roles: &[String], role: &str, checked: bool) -> Vec<String> {
    let mut next = normalize_roles(roles.to_vec());
    next.retain(|r| r != role);
    if checked {
        next.push(role.to_string());
    }
    normalize_roles(next)
}

#[component]
pub fn UsersTable(
    csrf_sig: RwSignal<Option<String>>,
    flash: RwSignal<Option<(bool, String)>>,
    on_changed: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();

    // Draft card (create user)
    let draft_pending = RwSignal::new(false);
    let draft_username = RwSignal::new(String::new());
    let draft_first_name = RwSignal::new(String::new());
    let draft_last_name = RwSignal::new(String::new());
    let draft_email = RwSignal::new(String::new());
    let draft_password = RwSignal::new(String::new());
    let draft_status = RwSignal::new("active".to_string());
    let draft_reset_required = RwSignal::new(true);
    let draft_role_user = RwSignal::new(true);
    let draft_role_admin = RwSignal::new(false);
    let draft_role_staff = RwSignal::new(false);

    // Existing users
    let rows = RwSignal::<Vec<RowHandle>>::new(Vec::new());
    let pending = RwSignal::<HashSet<(i64, PendingKind)>>::new(HashSet::new());
    let row_password_draft = RwSignal::<HashMap<i64, String>>::new(HashMap::new());

    // Search state (submit on Enter / button)
    let search_query = RwSignal::new(String::new());
    let search_pending = RwSignal::new(false);
    let searched_once = RwSignal::new(false);

    let csrf_ready = move || csrf_sig.read().is_some();
    let is_pending_row = move |id: i64| pending.with(|p| p.iter().any(|(pid, _)| *pid == id));

    let set_pending = move |id: i64, kind: PendingKind, on: bool| {
        pending.update(|p| {
            if on {
                p.insert((id, kind));
            } else {
                p.remove(&(id, kind));
            }
        });
    };

    let load = Callback::new({
        move |_| {
            let q = search_query.get_untracked().trim().to_string();
            searched_once.set(true);

            if q.chars().count() < 2 {
                rows.set(Vec::new());
                return;
            }

            flash.set(None);
            search_pending.set(true);

            leptos::task::spawn_local(async move {
                match admin_users_search(q).await {
                    Ok(list) => {
                        let mut mapped = list
                            .into_iter()
                            .map(|u| (u.id, ArcRwSignal::new(u)))
                            .collect::<Vec<_>>();
                        mapped.sort_by_key(|(id, _)| *id);
                        rows.set(mapped);
                    }
                    Err(e) => flash.set(Some((false, e.to_string()))),
                }
                search_pending.set(false);
            });
        }
    });

    let find_row = move |id: i64| -> Option<ArcRwSignal<AdminUserRow>> {
        rows.with(|rs| {
            rs.iter()
                .find(|(rid, _)| *rid == id)
                .map(|(_, row_sig)| row_sig.clone())
        })
    };

    let clear_draft = move |_| {
        if draft_pending.get_untracked() {
            return;
        }
        draft_username.set(String::new());
        draft_first_name.set(String::new());
        draft_last_name.set(String::new());
        draft_email.set(String::new());
        draft_password.set(String::new());
        draft_status.set("active".to_string());
        draft_reset_required.set(true);
        draft_role_user.set(true);
        draft_role_admin.set(false);
        draft_role_staff.set(false);
        flash.set(None);
    };

    let save_draft = {
        let load = load.clone();
        move |_| {
            if draft_pending.get_untracked() {
                return;
            }

            let Some(csrf) = csrf_sig.get_untracked() else {
                flash.set(Some((false, "msg_missing_csrf".to_string())));
                return;
            };

            let username = draft_username.get_untracked().trim().to_string();
            let first_name = draft_first_name.get_untracked().trim().to_string();
            let last_name = draft_last_name.get_untracked().trim().to_string();
            let email = draft_email.get_untracked().trim().to_string();
            let password = draft_password.get_untracked();
            let status = draft_status.get_untracked();
            let reset_required = draft_reset_required.get_untracked();

            if username.is_empty() {
                flash.set(Some((false, "err_username_required".to_string())));
                return;
            }
            if email.is_empty() || !email.contains('@') {
                flash.set(Some((false, "err_invalid_email".to_string())));
                return;
            }
            if password.len() < 10 {
                flash.set(Some((false, "err_password_too_short".to_string())));
                return;
            }
            if status != "active" && status != "disabled" {
                flash.set(Some((false, "err_invalid_status".to_string())));
                return;
            }

            let mut draft_roles = Vec::<String>::new();
            if draft_role_user.get_untracked() {
                draft_roles.push("user".to_string());
            }
            if draft_role_admin.get_untracked() {
                draft_roles.push("admin".to_string());
            }
            if draft_role_staff.get_untracked() {
                draft_roles.push("staff".to_string());
            }
            let draft_roles = normalize_roles(draft_roles);
            if draft_roles.is_empty() {
                flash.set(Some((false, "err_roles_required".to_string())));
                return;
            }

            let roles_csv = roles_to_csv(&draft_roles);

            draft_pending.set(true);
            flash.set(None);

            leptos::task::spawn_local(async move {
                match admin_users_create(
                    csrf,
                    username,
                    first_name,
                    last_name,
                    email,
                    password,
                    status,
                    roles_csv,
                    reset_required,
                )
                .await
                {
                    Ok(()) => {
                        draft_username.set(String::new());
                        draft_first_name.set(String::new());
                        draft_last_name.set(String::new());
                        draft_email.set(String::new());
                        draft_password.set(String::new());
                        draft_status.set("active".to_string());
                        draft_reset_required.set(true);
                        draft_role_user.set(true);
                        draft_role_admin.set(false);
                        draft_role_staff.set(false);

                        flash.set(Some((true, "msg_user_created".to_string())));
                        load.run(());
                        on_changed.run(());
                    }
                    Err(e) => flash.set(Some((false, e.to_string()))),
                }
                draft_pending.set(false);
            });
        }
    };

    let save_row = move |id: i64| {
        if is_pending_row(id) {
            return;
        }

        let Some(csrf) = csrf_sig.get_untracked() else {
            flash.set(Some((false, "msg_missing_csrf".to_string())));
            return;
        };

        let Some(row_sig) = find_row(id) else {
            return;
        };

        let row = row_sig.get_untracked();
        let username = row.username.trim().to_string();
        let first_name = row.first_name.trim().to_string();
        let last_name = row.last_name.trim().to_string();
        let email = row.email.trim().to_string();
        let status = row.status.trim().to_string();
        let reset_required = row.password_reset_required;
        let roles = normalize_roles(row.roles.clone());
        let new_password = row_password_draft
            .with_untracked(|m| m.get(&id).cloned().unwrap_or_default());

        if username.is_empty() {
            flash.set(Some((false, "err_username_required".to_string())));
            return;
        }
        if email.is_empty() || !email.contains('@') {
            flash.set(Some((false, "err_invalid_email".to_string())));
            return;
        }
        if status != "active" && status != "disabled" {
            flash.set(Some((false, "err_invalid_status".to_string())));
            return;
        }
        if roles.is_empty() {
            flash.set(Some((false, "err_roles_required".to_string())));
            return;
        }

        let roles_csv = roles_to_csv(&roles);

        set_pending(id, PendingKind::Save, true);
        flash.set(None);

        leptos::task::spawn_local(async move {
            let result = if new_password.trim().is_empty() {
                admin_users_update(
                    csrf,
                    id,
                    username,
                    first_name,
                    last_name,
                    email,
                    status,
                    roles_csv,
                    reset_required,
                )
                .await
            } else {
                admin_users_update_with_password(
                    csrf,
                    id,
                    username,
                    first_name,
                    last_name,
                    email,
                    status,
                    roles_csv,
                    reset_required,
                    new_password,
                )
                .await
            };

            set_pending(id, PendingKind::Save, false);

            match result {
                Ok(()) => {
                    row_password_draft.update(|m| {
                        m.insert(id, String::new());
                    });
                    flash.set(Some((true, "msg_user_updated".to_string())));
                    on_changed.run(());
                }
                Err(e) => flash.set(Some((false, e.to_string()))),
            }
        });
    };

    let on_delete = move |id: i64| {
        if is_pending_row(id) {
            return;
        }

        let Some(csrf) = csrf_sig.get_untracked() else {
            flash.set(Some((false, "msg_missing_csrf".to_string())));
            return;
        };

        let removed = rows.with_untracked(|rs| rs.iter().find(|(rid, _)| *rid == id).cloned());
        let Some(removed) = removed else {
            return;
        };

        rows.update(|rs| rs.retain(|(rid, _)| *rid != id));
        set_pending(id, PendingKind::Delete, true);

        leptos::task::spawn_local(async move {
            let result = admin_users_delete(csrf, id).await;
            set_pending(id, PendingKind::Delete, false);

            match result {
                Ok(()) => {
                    flash.set(Some((true, "msg_user_deleted".to_string())));
                    on_changed.run(());
                }
                Err(e) => {
                    rows.update(|rs| {
                        rs.push(removed);
                        rs.sort_by_key(|(rid, _)| *rid);
                    });
                    flash.set(Some((false, e.to_string())));
                }
            }
        });
    };

    view! {
        <section>
            <h2>{t!(i18n, admin.users)}</h2>

            <Show when=move || !csrf_ready()>
                <p style="color:#a35a00;">{t!(i18n, admin.msg_missing_csrf)}</p>
            </Show>

            <form
                on:submit=move |ev: leptos::ev::SubmitEvent| {
                    ev.prevent_default();
                    load.run(());
                }
                style="display:grid; gap:0.4rem; margin-bottom:1rem; justify-items:center;"
            >
                <label for="admin_user_search">
                    {move || format!(
                        "{} / {}",
                        td_string!(i18n.get_locale(), admin.username),
                        td_string!(i18n.get_locale(), admin.email)
                    )}
                </label>
                <div style="display:flex; gap:0.5rem; align-items:center; justify-content:center;">
                    <input
                        id="admin_user_search"
                        type="search"
                        autocomplete="off"
                        autocapitalize="none"
                        spellcheck="false"
                        bind:value=search_query
                        placeholder=move || format!(
                            "{} / {}",
                            td_string!(i18n.get_locale(), admin.username),
                            td_string!(i18n.get_locale(), admin.email)
                        )
                    />
                    <button
                        type="submit"
                        disabled=move || search_pending.get()
                    >
                        {move || if search_pending.get() {
                            t!(i18n, admin.working)()
                        } else {
                            t!(i18n, admin.search)()
                        }}
                    </button>
                </div>
                <Show when=move || search_query.with(|q| q.trim().chars().count() < 2)>
                    <small style="color:#666; text-align:center; display:block;">
                        {move || td_string!(i18n.get_locale(), admin.search_min_chars_hint).to_string()}
                    </small>
                </Show>
            </form>

            <Show
                when=move || {
                    searched_once.get()
                        && !search_pending.get()
                        && search_query.with(|q| q.trim().chars().count() >= 2)
                        && rows.with(|r| r.is_empty())
                }
            >
                <small style="color:#666;">
                    {t!(i18n, admin.search_no_results_create_below)}
                </small>
            </Show>

            // User cards (search results)
            <div style="display:grid; gap:0.9rem;">
                <For each=move || rows.get() key=|(id, _)| *id let:row>
                    {{
                        let row_id = row.0;
                        let row_sig = row.1;

                        let row_sig_username = row_sig.clone();
                        let row_sig_username_change = row_sig.clone();

                        let row_sig_first_name = row_sig.clone();
                        let row_sig_first_name_change = row_sig.clone();

                        let row_sig_last_name = row_sig.clone();
                        let row_sig_last_name_change = row_sig.clone();

                        let row_sig_email = row_sig.clone();
                        let row_sig_email_change = row_sig.clone();

                        let row_sig_status = row_sig.clone();
                        let row_sig_status_change = row_sig.clone();

                        let row_sig_reset = row_sig.clone();
                        let row_sig_reset_change = row_sig.clone();

                        let row_sig_roles_user = row_sig.clone();
                        let row_sig_roles_user_change = row_sig.clone();

                        let row_sig_roles_admin = row_sig.clone();
                        let row_sig_roles_admin_change = row_sig.clone();

                        let row_sig_roles_staff = row_sig.clone();
                        let row_sig_roles_staff_change = row_sig.clone();

                        let row_sig_created = row_sig.clone();
                        let row_sig_updated = row_sig.clone();

                        view! {
                            <article
                                style=move || {
                                    if is_pending_row(row_id) {
                                        "border:1px solid #d8d8d8; border-radius:8px; padding:0.9rem; background:#fff; opacity:0.65;"
                                    } else {
                                        "border:1px solid #d8d8d8; border-radius:8px; padding:0.9rem; background:#fff;"
                                    }
                                }
                            >
                                <div style="display:flex; justify-content:space-between; align-items:center; gap:0.75rem; margin-bottom:0.6rem;">
                                    <strong>{format!("{} {}", td_string!(i18n.get_locale(), admin.id), row_id)}</strong>
                                    <div style="display:flex; gap:0.5rem; align-items:center;">
                                        <button
                                            type="button"
                                            on:click=move |_| save_row(row_id)
                                            disabled=move || is_pending_row(row_id) || !csrf_ready()
                                        >
                                            {t!(i18n, admin.save_changes)}
                                        </button>
                                        <button
                                            type="button"
                                            on:click=move |_| on_delete(row_id)
                                            disabled=move || is_pending_row(row_id) || !csrf_ready()
                                        >
                                            {t!(i18n, admin.delete)}
                                        </button>
                                        <Show when=move || is_pending_row(row_id)>
                                            <small>{t!(i18n, admin.working)}</small>
                                        </Show>
                                    </div>
                                </div>

                                <div style="display:grid; gap:0.75rem;">
                                    <div style="display:grid; gap:0.25rem;">
                                        <label>{t!(i18n, admin.username)}</label>
                                        <input
                                            type="text"
                                            prop:value=move || row_sig_username.with(|r| r.username.clone())
                                            on:input:target=move |ev| {
                                                let v = ev.target().value();
                                                row_sig_username_change.update(|r| r.username = v);
                                            }
                                            disabled=move || is_pending_row(row_id) || !csrf_ready()
                                        />
                                    </div>

                                    <div style="display:grid; grid-template-columns: 1fr 1fr; gap:0.6rem;">
                                        <div style="display:grid; gap:0.25rem;">
                                            <label>{t!(i18n, admin.first_name)}</label>
                                            <input
                                                type="text"
                                                prop:value=move || row_sig_first_name.with(|r| r.first_name.clone())
                                                on:input:target=move |ev| {
                                                    let v = ev.target().value();
                                                    row_sig_first_name_change.update(|r| r.first_name = v);
                                                }
                                                disabled=move || is_pending_row(row_id) || !csrf_ready()
                                            />
                                        </div>
                                        <div style="display:grid; gap:0.25rem;">
                                            <label>{t!(i18n, admin.last_name)}</label>
                                            <input
                                                type="text"
                                                prop:value=move || row_sig_last_name.with(|r| r.last_name.clone())
                                                on:input:target=move |ev| {
                                                    let v = ev.target().value();
                                                    row_sig_last_name_change.update(|r| r.last_name = v);
                                                }
                                                disabled=move || is_pending_row(row_id) || !csrf_ready()
                                            />
                                        </div>
                                    </div>

                                    <div style="display:grid; gap:0.25rem;">
                                        <label>{t!(i18n, admin.email)}</label>
                                        <input
                                            type="email"
                                            prop:value=move || row_sig_email.with(|r| r.email.clone())
                                            on:input:target=move |ev| {
                                                let v = ev.target().value();
                                                row_sig_email_change.update(|r| r.email = v);
                                            }
                                            disabled=move || is_pending_row(row_id) || !csrf_ready()
                                        />
                                    </div>

                                    <div style="display:grid; gap:0.25rem; max-width:18rem;">
                                        <label>{t!(i18n, admin.password)}</label>
                                        <input
                                            type="password"
                                            prop:value=move || {
                                                row_password_draft
                                                    .with(|m| m.get(&row_id).cloned().unwrap_or_default())
                                            }
                                            on:input:target=move |ev| {
                                                let v = ev.target().value();
                                                row_password_draft.update(|m| {
                                                    m.insert(row_id, v);
                                                });
                                            }
                                            placeholder=move || td_string!(i18n.get_locale(), admin.password).to_string()
                                            disabled=move || is_pending_row(row_id) || !csrf_ready()
                                        />
                                    </div>

                                    <div style="display:grid; grid-template-columns: 1fr 1fr; gap:0.6rem; max-width:22rem;">
                                        <div style="display:grid; gap:0.25rem;">
                                            <label>{t!(i18n, admin.status)}</label>
                                            <select
                                                prop:value=move || row_sig_status.with(|r| r.status.clone())
                                                on:change:target=move |ev| {
                                                    let value = ev.target().value();
                                                    row_sig_status_change.update(|r| {
                                                        r.status = if value == "disabled" {
                                                            "disabled".to_string()
                                                        } else {
                                                            "active".to_string()
                                                        };
                                                    });
                                                }
                                                disabled=move || is_pending_row(row_id) || !csrf_ready()
                                            >
                                                <option value="active">{t!(i18n, admin.active)}</option>
                                                <option value="disabled">{t!(i18n, admin.disabled)}</option>
                                            </select>
                                        </div>
                                        <div style="display:grid; gap:0.25rem;">
                                            <label>{t!(i18n, admin.reset_required)}</label>
                                            <select
                                                prop:value=move || {
                                                    if row_sig_reset.with(|r| r.password_reset_required) {
                                                        "true".to_string()
                                                    } else {
                                                        "false".to_string()
                                                    }
                                                }
                                                on:change:target=move |ev| {
                                                    let required = ev.target().value() == "true";
                                                    row_sig_reset_change.update(|r| r.password_reset_required = required);
                                                }
                                                disabled=move || is_pending_row(row_id) || !csrf_ready()
                                            >
                                                <option value="true">{t!(i18n, admin.yes)}</option>
                                                <option value="false">{t!(i18n, admin.no)}</option>
                                            </select>
                                        </div>
                                    </div>

                                    <div style="display:grid; gap:0.25rem;">
                                        <label>{t!(i18n, admin.roles)}</label>
                                        <div style="display:flex; gap:0.7rem; flex-wrap:wrap;">
                                            <label style="display:flex; align-items:center; gap:0.25rem;">
                                                <input
                                                    type="checkbox"
                                                    prop:checked=move || row_sig_roles_user.with(|r| r.roles.iter().any(|x| x.eq_ignore_ascii_case("user")))
                                                    on:change:target=move |ev| {
                                                        let checked = ev.target().checked();
                                                        row_sig_roles_user_change.update(|r| {
                                                            r.roles = toggle_role(&r.roles, "user", checked);
                                                            r.is_admin = r.roles.iter().any(|x| x.eq_ignore_ascii_case("admin"));
                                                        });
                                                    }
                                                    disabled=move || is_pending_row(row_id) || !csrf_ready()
                                                />
                                                {t!(i18n, admin.role_user)}
                                            </label>

                                            <label style="display:flex; align-items:center; gap:0.25rem;">
                                                <input
                                                    type="checkbox"
                                                    prop:checked=move || row_sig_roles_admin.with(|r| r.roles.iter().any(|x| x.eq_ignore_ascii_case("admin")))
                                                    on:change:target=move |ev| {
                                                        let checked = ev.target().checked();
                                                        row_sig_roles_admin_change.update(|r| {
                                                            r.roles = toggle_role(&r.roles, "admin", checked);
                                                            r.is_admin = r.roles.iter().any(|x| x.eq_ignore_ascii_case("admin"));
                                                        });
                                                    }
                                                    disabled=move || is_pending_row(row_id) || !csrf_ready()
                                                />
                                                {t!(i18n, admin.role_admin)}
                                            </label>

                                            <label style="display:flex; align-items:center; gap:0.25rem;">
                                                <input
                                                    type="checkbox"
                                                    prop:checked=move || row_sig_roles_staff.with(|r| r.roles.iter().any(|x| x.eq_ignore_ascii_case("staff")))
                                                    on:change:target=move |ev| {
                                                        let checked = ev.target().checked();
                                                        row_sig_roles_staff_change.update(|r| {
                                                            r.roles = toggle_role(&r.roles, "staff", checked);
                                                            r.is_admin = r.roles.iter().any(|x| x.eq_ignore_ascii_case("admin"));
                                                        });
                                                    }
                                                    disabled=move || is_pending_row(row_id) || !csrf_ready()
                                                />
                                                {t!(i18n, admin.role_staff)}
                                            </label>
                                        </div>
                                    </div>

                                    <small style="color:#666;">
                                        {move || format!(
                                            "{}: {} | {}: {}",
                                            td_string!(i18n.get_locale(), admin.created_at),
                                            row_sig_created.with(|r| r.created_at.clone()),
                                            td_string!(i18n.get_locale(), admin.updated_at),
                                            row_sig_updated.with(|r| r.updated_at.clone())
                                        )}
                                    </small>
                                </div>
                            </article>
                        }
                    }}
                </For>
            </div>

            <Show
                when=move || searched_once.get() && !search_pending.get() && rows.with(|r| r.is_empty())
            >
                <p style="color:#666; margin-top:0.75rem; margin-bottom:0.75rem;">
                    {move || format!(
                        "{} — {}",
                        td_string!(i18n.get_locale(), admin.msg_no_user_selected),
                        td_string!(i18n.get_locale(), admin.create_user)
                    )}
                </p>
            </Show>

            // Create user card (shown below search results)
            <article style="border:1px solid #d8d8d8; border-radius:8px; padding:0.9rem; margin-top:1rem; background:#fafafa;">
                <h3 style="margin-top:0;">{t!(i18n, admin.create_user)}</h3>

                <div style="display:grid; gap:0.75rem;">
                    <div style="display:grid; gap:0.25rem;">
                        <label for="draft_username">{t!(i18n, admin.username)}</label>
                        <input
                            id="draft_username"
                            name="draft_account_username"
                            type="text"
                            autocomplete="off"
                            autocapitalize="none"
                            spellcheck="false"
                            bind:value=draft_username
                            disabled=move || draft_pending.get()
                        />
                    </div>

                    <div style="display:grid; grid-template-columns: 1fr 1fr; gap:0.6rem;">
                        <div style="display:grid; gap:0.25rem;">
                            <label for="draft_first_name">{t!(i18n, admin.first_name)}</label>
                            <input
                                id="draft_first_name"
                                type="text"
                                bind:value=draft_first_name
                                disabled=move || draft_pending.get()
                            />
                        </div>
                        <div style="display:grid; gap:0.25rem;">
                            <label for="draft_last_name">{t!(i18n, admin.last_name)}</label>
                            <input
                                id="draft_last_name"
                                type="text"
                                bind:value=draft_last_name
                                disabled=move || draft_pending.get()
                            />
                        </div>
                    </div>

                    <div style="display:grid; gap:0.25rem;">
                        <label for="draft_email">{t!(i18n, admin.email)}</label>
                        <input
                            id="draft_email"
                            type="email"
                            bind:value=draft_email
                            disabled=move || draft_pending.get()
                        />
                    </div>

                    <div style="display:grid; gap:0.25rem; max-width:18rem;">
                        <label for="draft_password">{t!(i18n, admin.password)}</label>
                        <input
                            id="draft_password"
                            type="password"
                            bind:value=draft_password
                            disabled=move || draft_pending.get()
                        />
                    </div>

                    <div style="display:grid; grid-template-columns: 1fr 1fr; gap:0.6rem; max-width:22rem;">
                        <div style="display:grid; gap:0.25rem;">
                            <label>{t!(i18n, admin.status)}</label>
                            <select
                                prop:value=move || draft_status.get()
                                on:change:target=move |ev| draft_status.set(ev.target().value())
                                disabled=move || draft_pending.get()
                            >
                                <option value="active">{t!(i18n, admin.active)}</option>
                                <option value="disabled">{t!(i18n, admin.disabled)}</option>
                            </select>
                        </div>
                        <div style="display:grid; gap:0.25rem;">
                            <label>{t!(i18n, admin.reset_required)}</label>
                            <select
                                prop:value=move || if draft_reset_required.get() { "true".to_string() } else { "false".to_string() }
                                on:change:target=move |ev| draft_reset_required.set(ev.target().value() == "true")
                                disabled=move || draft_pending.get()
                            >
                                <option value="true">{t!(i18n, admin.yes)}</option>
                                <option value="false">{t!(i18n, admin.no)}</option>
                            </select>
                        </div>
                    </div>

                    <div style="display:grid; gap:0.25rem;">
                        <label>{t!(i18n, admin.roles)}</label>
                        <div style="display:flex; gap:0.7rem; flex-wrap:wrap;">
                            <label style="display:flex; align-items:center; gap:0.25rem;">
                                <input
                                    type="checkbox"
                                    bind:checked=draft_role_user
                                    disabled=move || draft_pending.get()
                                />
                                {t!(i18n, admin.role_user)}
                            </label>
                            <label style="display:flex; align-items:center; gap:0.25rem;">
                                <input
                                    type="checkbox"
                                    bind:checked=draft_role_admin
                                    disabled=move || draft_pending.get()
                                />
                                {t!(i18n, admin.role_admin)}
                            </label>
                            <label style="display:flex; align-items:center; gap:0.25rem;">
                                <input
                                    type="checkbox"
                                    bind:checked=draft_role_staff
                                    disabled=move || draft_pending.get()
                                />
                                {t!(i18n, admin.role_staff)}
                            </label>
                        </div>
                    </div>

                    <div style="display:flex; gap:0.5rem; align-items:center;">
                        <button
                            type="button"
                            on:click=save_draft
                            disabled=move || draft_pending.get() || !csrf_ready()
                        >
                            {move || if draft_pending.get() {
                                t!(i18n, admin.working)()
                            } else {
                                t!(i18n, admin.save_changes)()
                            }}
                        </button>
                        <button
                            type="button"
                            on:click=clear_draft
                            disabled=move || draft_pending.get()
                        >
                            {t!(i18n, admin.cancel)}
                        </button>
                    </div>
                </div>
            </article>
        </section>
    }
}
