use crossterm::{
    event::{self, Event, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::Write;

pub fn run_key_debug() {
    let mut log = std::fs::File::create("/tmp/farx_keys.log").unwrap();

    enable_raw_mode().unwrap();

    print!("Key debug mode. Press keys. Ctrl+Q to quit.\r\n");
    print!("Output goes to /tmp/farx_keys.log\r\n");
    print!("---\r\n");

    writeln!(log, "farx key debug started").unwrap();
    log.flush().unwrap();

    loop {
        if let Ok(evt) = event::read() {
            match evt {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    let msg = format!(
                        "code={:?}  mod={:?}  state={:?}",
                        key.code, key.modifiers, key.state
                    );

                    // Write to both file and screen
                    writeln!(log, "{}", msg).unwrap();
                    log.flush().unwrap();

                    print!("{}\r\n", msg);

                    if key.code == crossterm::event::KeyCode::Char('q')
                        && key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    disable_raw_mode().unwrap();
    println!("\r\nDone. Log at /tmp/farx_keys.log\r");
}
