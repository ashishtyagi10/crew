fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("crew-inbox-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

const DOWNLOAD: &str = r#"{"installed":{"client_id":"123-abc.apps.googleusercontent.com","project_id":"crew-inbox","auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token","client_secret":"GOCSPX-xyz","redirect_uris":["http://localhost"]}}"#;

#[test]
fn the_consoles_download_and_a_bare_pair_both_read_as_a_client() {
    let c = super::client_from_json(DOWNLOAD).expect("the download");
    assert_eq!(
        (c.id.as_str(), c.secret.as_str()),
        ("123-abc.apps.googleusercontent.com", "GOCSPX-xyz")
    );
    let bare = r#" {"client_id": "9.apps.googleusercontent.com", "client_secret": "s"} "#;
    assert_eq!(
        super::client_from_json(bare).unwrap().id,
        "9.apps.googleusercontent.com"
    );
    for bad in [
        "",
        "{}",
        r#"{"installed":{"client_id":"x"}}"#,
        r#"{"client_id":"","client_secret":"s"}"#,
    ] {
        assert!(super::client_from_json(bad).is_none(), "{bad:?}");
    }
}

#[test]
fn the_newest_download_in_the_folder_is_found() {
    let d = dir("dl");
    std::fs::write(d.join("notes.json"), DOWNLOAD).unwrap();
    assert!(
        super::downloaded_client(&d).is_none(),
        "only client_secret_*.json counts"
    );
    std::fs::write(
        d.join("client_secret_old.json"),
        DOWNLOAD.replace("123-abc", "old"),
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(d.join("client_secret_new.json"), DOWNLOAD).unwrap();
    let (path, c) = super::downloaded_client(&d).unwrap();
    assert!(path.ends_with("client_secret_new.json"), "{path:?}");
    assert!(c.id.starts_with("123-abc"));
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_grant_round_trips_through_a_private_file_and_never_prints_its_token() {
    let d = dir("grant");
    let g = super::Grant {
        client: super::client_from_json(DOWNLOAD).unwrap(),
        refresh_token: "1//refresh-secret".into(),
        email: "ann@gmail.com".into(),
    };
    let p = d.join("gmail.json");
    super::save_to(&p, &g).unwrap();
    assert!(super::load_from(&p) == Some(g.clone()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&p).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let shown = format!("{g:?}");
    assert!(
        !shown.contains("refresh-secret") && !shown.contains("GOCSPX"),
        "{shown}"
    );
    let _ = std::fs::remove_dir_all(d);
}
