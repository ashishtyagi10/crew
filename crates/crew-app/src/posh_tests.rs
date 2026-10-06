use super::*;
use std::collections::HashMap;

#[test]
fn powershell_is_known_by_its_file_name() {
    for shell in [
        "powershell.exe",
        r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
        r"C:\Program Files\PowerShell\7\PWSH.EXE",
        "/usr/local/bin/pwsh",
    ] {
        assert!(is_powershell(shell), "{shell}");
    }
    for shell in [
        "cmd.exe",
        r"C:\Windows\System32\cmd.exe",
        "/bin/zsh",
        "bash",
        "",
    ] {
        assert!(!is_powershell(shell), "{shell}");
    }
}

/// `PATH` first, then where the installers leave it — winget's alias first —
/// and nothing when it is nowhere.
#[test]
fn oh_my_posh_is_found_on_path_then_where_installers_leave_it() {
    let path = std::env::join_paths(["/a", "/b"])
        .unwrap()
        .into_string()
        .unwrap();
    let vars: HashMap<&str, String> = [
        ("PATH", path),
        ("LOCALAPPDATA", "/local".to_string()),
        ("USERPROFILE", "/home".to_string()),
    ]
    .into();
    let env = |k: &str| vars.get(k).cloned();
    let at = |hits: &'static [&'static str]| {
        find(env, move |p: &Path| hits.iter().any(|h| p == Path::new(h)))
    };
    let alias: PathBuf = ["/local", "Microsoft", "WindowsApps", EXE].iter().collect();
    let scoop: PathBuf = ["/home", "scoop", "shims", EXE].iter().collect();
    let on_b = Path::new("/b").join(EXE);
    assert_eq!(
        find(env, |p: &Path| p == on_b || p == alias),
        Some(on_b.clone()),
        "PATH wins"
    );
    assert_eq!(find(env, |p: &Path| p == alias || p == scoop), Some(alias));
    assert_eq!(find(env, |p: &Path| p == scoop), Some(scoop));
    assert_eq!(at(&[]), None);
}

#[test]
fn a_found_oh_my_posh_inits_over_the_default_prompt_only_in_memory() {
    let s = script(
        Some(Path::new(r"C:\Users\o'neil\oh-my-posh.exe")),
        false,
        Path::new(r"C:\Users\o'neil\AppData\Roaming\crew\crew.omp.json"),
    );
    assert!(
        s.contains(r"$__crewPosh = 'C:\Users\o''neil\oh-my-posh.exe'"),
        "{s}"
    );
    assert!(
        s.contains(r"--config 'C:\Users\o''neil\AppData\Roaming\crew\crew.omp.json' --eval"),
        "{s}"
    );
    assert!(
        s.contains("-like '*nestedPromptLevel + 1*'"),
        "only over the default prompt: {s}"
    );
    assert!(!s.contains("winget"), "nothing to install: {s}");
    // The prompt reads `$?`: a last command that errored (removing a
    // variable that was never set) opened every pane on a red arrow.
    assert!(
        s.trim_end().ends_with("Remove-Variable -Name __crew*"),
        "{s}"
    );
}

#[test]
fn the_install_runs_only_when_asked_and_only_through_winget() {
    let cfg = Path::new("crew.omp.json");
    let s = script(None, true, cfg);
    assert!(s.contains("$__crewPosh = $null"), "{s}");
    assert!(
        s.contains("winget install --id JanDeDobbeleer.OhMyPosh --exact --source winget"),
        "{s}"
    );
    assert!(s.contains(r"'Microsoft\WindowsApps\oh-my-posh.exe'"), "{s}");
    assert!(!script(None, false, cfg).contains("winget"));
}

/// `-EncodedCommand` is base64 of UTF-16LE — anything else and PowerShell
/// runs garbage or nothing.
#[test]
fn the_script_reaches_powershell_as_utf16_base64() {
    use base64::Engine as _;
    let s = script(None, true, Path::new("C:\\crew\\crew.omp.json"));
    let args = pane_args(&s);
    assert_eq!(args[..3], ["-NoLogo", "-NoExit", "-EncodedCommand"]);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&args[3])
        .unwrap();
    let units: Vec<u16> = bytes
        .chunks(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    assert_eq!(String::from_utf16(&units).unwrap(), s);
}

/// The theme is oh-my-posh's own format, follows crew's palette (ANSI
/// names, never a fixed hex) and draws with no Nerd Font icon.
#[test]
fn the_theme_is_robbyrussell_in_crew_s_palette() {
    let v: serde_json::Value = serde_json::from_str(THEME).unwrap();
    assert_eq!(v["version"], 4);
    let segs = v["blocks"][0]["segments"].as_array().unwrap();
    let kinds: Vec<_> = segs.iter().map(|s| s["type"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["text", "path", "git"]);
    assert!(!THEME.contains('#'), "a fixed colour ignores the theme");
    let text = v.to_string();
    assert!(text.contains('\u{279c}') && text.contains('\u{2717}'));
    assert!(
        !text.chars().any(|c| ('\u{e000}'..='\u{f8ff}').contains(&c)),
        "a Nerd Font icon"
    );
}

/// The theme lands beside crew's config; the install is armed once, when
/// oh-my-posh is nowhere, and never again — not even after the user
/// uninstalls it; a found oh-my-posh arms nothing.
#[test]
fn the_install_is_armed_once_and_the_theme_kept_current() {
    use base64::Engine as _;
    let dir = std::env::temp_dir().join(format!("crew-posh-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let decoded = |args: &[String]| {
        let b = base64::engine::general_purpose::STANDARD
            .decode(&args[3])
            .unwrap();
        let u: Vec<u16> = b
            .chunks(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16(&u).unwrap()
    };
    let nowhere = |_: &str| None;
    let first = args_in(&dir, nowhere, |_: &Path| false);
    assert_eq!(
        std::fs::read_to_string(dir.join("crew.omp.json")).unwrap(),
        THEME
    );
    assert!(
        decoded(&first).contains("winget install"),
        "first pane installs"
    );
    let again = args_in(&dir, nowhere, |_: &Path| false);
    assert!(!decoded(&again).contains("winget"), "only ever once");
    std::fs::write(dir.join("crew.omp.json"), "{}").unwrap();
    let path = std::env::join_paths(["/bin"])
        .unwrap()
        .into_string()
        .unwrap();
    let found = args_in(
        &dir,
        |k: &str| (k == "PATH").then(|| path.clone()),
        |p: &Path| p.ends_with(EXE),
    );
    let s = decoded(&found);
    assert!(
        s.contains("oh-my-posh.exe'") && !s.contains("winget"),
        "{s}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("crew.omp.json")).unwrap(),
        THEME,
        "an older theme is replaced"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
