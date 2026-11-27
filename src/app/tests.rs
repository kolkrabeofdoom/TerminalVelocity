use crate::*;
#[test]
fn command_palette_filter_and_background_tab_notifications() {
    assert!(palette_matches_query(
        "Neuer Tab: Antigravity CLI",
        "antigravity tab"
    ));
    assert!(palette_matches_query(
        "Verbinden (Nebeneinander): ★ Prod · SSH · root@server",
        "prod ssh"
    ));
    assert!(!palette_matches_query(
        "Neuer Tab: PowerShell",
        "antigravity"
    ));

    let mut workspace = Workspace::default();
    let mut sessions = HashMap::new();
    sessions.insert(
        1,
        Session {
            generation: 10,
            cwd_token: model::new_cwd_token(),
            backend: None,
            error: None,
            exited: false,
            notified: false,
        },
    );
    sessions.insert(
        2,
        Session {
            generation: 20,
            cwd_token: model::new_cwd_token(),
            backend: None,
            error: None,
            exited: false,
            notified: false,
        },
    );
    // Active pane (1) does not set notification badge on bell.
    handle_session_event(&mut workspace, &mut sessions, 10, PtyEvent::Bell, Some(1));
    assert!(!sessions[&1].notified);
    // Inactive pane (2) sets notification badge on bell and exit.
    handle_session_event(&mut workspace, &mut sessions, 20, PtyEvent::Bell, Some(1));
    assert!(sessions[&2].notified);
    sessions.get_mut(&2).unwrap().notified = false;
    handle_session_event(&mut workspace, &mut sessions, 20, PtyEvent::Exit, Some(1));
    assert!(sessions[&2].notified);
    assert!(sessions[&2].exited);
}
