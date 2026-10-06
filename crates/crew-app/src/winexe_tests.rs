use super::*;
use std::collections::HashMap;

fn vars(path: &[&str]) -> HashMap<&'static str, String> {
    let path = std::env::join_paths(path).unwrap().into_string().unwrap();
    [
        ("PATH", path),
        ("ProgramFiles", "/pf".to_string()),
        ("LOCALAPPDATA", "/local".to_string()),
        ("USERPROFILE", "/home".to_string()),
    ]
    .into()
}

/// PowerShell 7 on `PATH` wins; off it, the MSI's folder, then the
/// winget/Store alias; nowhere is `None`.
#[test]
fn pwsh_is_found_on_path_then_where_it_installs() {
    let v = vars(&["/a", "/b"]);
    let env = |k: &str| v.get(k).cloned();
    let msi: PathBuf = ["/pf", "PowerShell", "7", "pwsh.exe"].iter().collect();
    let alias: PathBuf = ["/local", "Microsoft", "WindowsApps", "pwsh.exe"]
        .iter()
        .collect();
    let tool: PathBuf = ["/home", ".dotnet", "tools", "pwsh.exe"].iter().collect();
    let on_b = Path::new("/b").join("pwsh.exe");
    assert_eq!(pwsh(env, |p: &Path| p == on_b || p == msi), Some(on_b));
    assert_eq!(pwsh(env, |p: &Path| p == msi || p == alias), Some(msi));
    assert_eq!(pwsh(env, |p: &Path| p == alias || p == tool), Some(alias));
    assert_eq!(pwsh(env, |p: &Path| p == tool), Some(tool));
    assert_eq!(pwsh(env, |_: &Path| false), None);
}

/// `$SHELL` is the user's word; then PowerShell 7; then Windows PowerShell.
#[test]
fn a_pane_opens_pwsh_7_when_installed() {
    let seven = Some(PathBuf::from(r"C:\Program Files\PowerShell\7\pwsh.exe"));
    assert_eq!(
        pane_shell(None, seven.clone()),
        r"C:\Program Files\PowerShell\7\pwsh.exe"
    );
    assert_eq!(pane_shell(None, None), "powershell.exe");
    assert_eq!(pane_shell(Some(String::new()), None), "powershell.exe");
    assert_eq!(
        pane_shell(Some(r"C:\Git\bin\bash.exe".into()), seven),
        r"C:\Git\bin\bash.exe"
    );
}

#[test]
fn present_sees_files_not_folders_or_nothing() {
    let dir = std::env::temp_dir();
    let file = dir.join(format!("crew-winexe-{}", std::process::id()));
    std::fs::write(&file, "").unwrap();
    assert!(present(&file));
    assert!(!present(&dir));
    std::fs::remove_file(&file).unwrap();
    assert!(!present(&file));
}
