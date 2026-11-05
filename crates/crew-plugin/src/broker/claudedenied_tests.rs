//! Through a real child: a run whose settings refused a tool says which
//! one as smith's verdict row (`Usage::refused`), and the reply stays the
//! agent's own words.
use std::time::Duration;

use super::tests::{agent, fake};
use crate::broker::adapter::Adapter;

#[cfg(unix)]
#[test]
fn a_refused_tool_is_named_under_the_reply() {
    let (dir, path) = fake(concat!(
        "#!/bin/sh\necho '{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,",
        "\"result\":\"I could not write it.\",\"permission_denials\":[{\"tool_name\":\"Write\",",
        "\"tool_use_id\":\"t1\",\"tool_input\":{\"file_path\":\"/x/hello.txt\"}}]}'\n",
    ));
    let (said, used) = agent(&path, Duration::ZERO)
        .call_with_usage("hi", Duration::from_secs(5))
        .unwrap();
    assert_eq!(
        said, "I could not write it.",
        "the reply is the agent's alone"
    );
    assert!(
        used.refused
            .starts_with("! not allowed: Write /x/hello.txt \u{2014} allow"),
        "{}",
        used.refused
    );
    let _ = std::fs::remove_dir_all(&dir);
    let (dir, path) = fake(
        "#!/bin/sh\necho '{\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"permission_denials\":[]}'\n",
    );
    let (said, used) = agent(&path, Duration::ZERO)
        .call_with_usage("hi", Duration::from_secs(5))
        .unwrap();
    assert_eq!((said.as_str(), used.refused.as_str()), ("done", ""));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A refused file inside the project is named from the project, not from
/// the root: the absolute path alone filled the row (live probe).
#[cfg(unix)]
#[test]
fn a_refused_file_in_the_project_is_named_relative_to_it() {
    let here = std::env::current_dir().unwrap();
    let (dir, path) = fake(&format!(
        "#!/bin/sh\necho '{{\"type\":\"result\",\"subtype\":\"success\",\"result\":\"no\",\
         \"permission_denials\":[{{\"tool_name\":\"Write\",\"tool_input\":{{\"file_path\":\
         \"{}/notes.txt\"}}}}]}}'\n",
        here.display()
    ));
    let (_, used) = agent(&path, Duration::ZERO)
        .call_with_usage("hi", Duration::from_secs(5))
        .unwrap();
    assert!(
        used.refused
            .starts_with("! not allowed: Write notes.txt \u{2014}"),
        "{}",
        used.refused
    );
    let _ = std::fs::remove_dir_all(&dir);
}
