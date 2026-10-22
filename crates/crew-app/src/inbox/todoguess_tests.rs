//! The guesses held to the corpus (`todo_corpus.toml`), as the goal sets it:
//! no wrong project and no assignee off the thread on any mail, no due date
//! on a mail that states none, and the right one on nine in ten that do.
use serde::Deserialize;

use super::{guess, Known};
use crate::inbox::api::Mail;
use crate::inbox::draft::Reply;
use crate::inbox::wire::Thread;
use crate::inbox::Reading;

#[derive(Deserialize)]
struct Corpus {
    case: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    subject: String,
    #[serde(default)]
    want_project: String,
    #[serde(default)]
    want_assignee: String,
    #[serde(default)]
    want_due: String,
    #[serde(default)]
    want_title: String,
    mail: Vec<CaseMail>,
}

#[derive(Deserialize)]
struct CaseMail {
    from: String,
    to: String,
    #[serde(default)]
    cc: String,
    date: String,
    text: String,
    #[serde(default)]
    calendar: String,
}

fn known() -> Known {
    let v = |s: &[&str]| s.iter().map(|s| s.to_string()).collect();
    Known {
        projects: v(&["crew", "rdactd", "hive"]),
        folders: v(&["crew", "rdactd", "hive", "docs", "api"]),
        people: v(&["bo", "ann"]),
        me: "sam@x.org".into(),
    }
}

fn reading(c: &Case) -> Reading {
    let mails = c
        .mail
        .iter()
        .map(|m| Mail {
            from: m.from.clone(),
            date: m.date.clone(),
            text: m.text.clone(),
            calendar: (!m.calendar.is_empty()).then(|| m.calendar.clone()),
            reply: Reply {
                to: m.to.clone(),
                cc: m.cc.clone(),
                ..Reply::default()
            },
            ..Mail::default()
        })
        .collect();
    Reading {
        mails: Some(mails),
        ..Reading::new(Thread {
            subject: c.subject.clone(),
            ..Thread::default()
        })
    }
}

#[test]
fn the_card_guesses_the_corpus_as_the_goal_asks() {
    let corpus: Corpus = toml::from_str(include_str!("todo_corpus.toml")).unwrap();
    let (mut stated, mut right, mut wrong) = (0, 0, Vec::new());
    for c in &corpus.case {
        let [title, project, who, due] = guess(&reading(c), &known());
        if project.text != c.want_project {
            wrong.push(format!(
                "{}: project {:?}, not {:?}",
                c.name, project.text, c.want_project
            ));
        }
        if who.text != c.want_assignee {
            wrong.push(format!(
                "{}: assignee {:?}, not {:?}",
                c.name, who.text, c.want_assignee
            ));
        }
        if !c.want_title.is_empty() && title.text != c.want_title {
            wrong.push(format!(
                "{}: title {:?}, not {:?}",
                c.name, title.text, c.want_title
            ));
        }
        match c.want_due.as_str() {
            "" if !due.text.is_empty() => wrong.push(format!(
                "{}: due {:?} where it states none",
                c.name, due.text
            )),
            "" => {}
            want => {
                stated += 1;
                if due.text == want {
                    right += 1;
                } else {
                    eprintln!("{}: due {:?}, not {want:?}", c.name, due.text);
                }
            }
        }
    }
    assert!(corpus.case.len() >= 25, "the corpus is about 25 mails");
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    assert!(
        right * 10 >= stated * 9,
        "due dates right on {right} of {stated}"
    );
}

/// Every guess says why, and a blank one says nothing.
#[test]
fn a_guess_says_why_and_a_blank_says_nothing() {
    let corpus: Corpus = toml::from_str(include_str!("todo_corpus.toml")).unwrap();
    for c in &corpus.case {
        for g in &guess(&reading(c), &known())[1..] {
            assert_eq!(g.text.is_empty(), g.why.is_empty(), "{}: {g:?}", c.name);
        }
    }
}
