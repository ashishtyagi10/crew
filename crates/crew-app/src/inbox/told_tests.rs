use super::told;

/// A Calendar answer that failed keeps saying it did not go.
#[test]
fn a_failed_answer_keeps_its_head() {
    assert_eq!(
        told("not answered: could not reach Gmail: timed out".into()),
        "not answered: Gmail can\u{2019}t be reached from here"
    );
    assert_eq!(
        told("not added: Google Calendar 409: The requested identifier already exists".into()),
        "not added: Google Calendar 409: The requested identifier already exists"
    );
}

/// Google Calendar's failures are named as its own.
#[test]
fn google_calendar_is_not_called_gmail() {
    assert_eq!(
        told("not answered: Google Calendar 429: Rate Limit Exceeded".into()),
        "not answered: Google Calendar asked crew to slow down"
    );
    assert_eq!(
        told("could not reach Google Calendar: dns error".into()),
        "Google Calendar can\u{2019}t be reached from here"
    );
}

/// Gmail's own failures read as before.
#[test]
fn gmail_failures_read_as_before() {
    assert_eq!(
        told("could not reach Gmail: dns error".into()),
        "Gmail can\u{2019}t be reached from here"
    );
    assert_eq!(told("Gmail 418: teapot".into()), "Gmail 418: teapot");
}

/// A grant Google no longer honours reads as the sign-in it is.
#[test]
fn an_invalid_grant_reads_as_a_turned_down_sign_in() {
    assert_eq!(
        told("not added: invalid_grant: Token has been expired or revoked.".into()),
        "not added: Gmail turned down crew\u{2019}s sign-in"
    );
}
