//! Through a real child: a run whose settings refused a tool says which
//! one under its reply.
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
    let said = agent(&path, Duration::ZERO)
        .call("hi", Duration::from_secs(5))
        .unwrap();
    assert!(
        said.starts_with(
            "I could not write it.\n\nClaude Code was not allowed to: Write /x/hello.txt."
        ),
        "{said}"
    );
    let _ = std::fs::remove_dir_all(&dir);
    let (dir, path) = fake(
        "#!/bin/sh\necho '{\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"permission_denials\":[]}'\n",
    );
    let said = agent(&path, Duration::ZERO).call("hi", Duration::from_secs(5));
    assert_eq!(said.unwrap(), "done");
    let _ = std::fs::remove_dir_all(&dir);
}
