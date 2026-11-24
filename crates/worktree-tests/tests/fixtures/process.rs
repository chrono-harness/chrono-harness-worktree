use std::{
    env,
    io::{Read, Write},
    process::{Command, Stdio},
    time::Duration,
};
fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("streams") => {
            let mut bytes = vec![];
            std::io::stdin().read_to_end(&mut bytes).unwrap();
            std::io::stdout().write_all(&bytes).unwrap();
            std::io::stderr().write_all(&[0xff, 0xfe]).unwrap();
            std::process::exit(7);
        }
        Some("burst") => {
            std::io::stdout().write_all(&[0xab; 4096]).unwrap();
        }
        Some("wait") => std::thread::sleep(Duration::from_secs(10)),
        Some("leave-child") => {
            let exe = env::current_exe().unwrap();
            Command::new(exe)
                .args(["finish-marker", &args.next().unwrap()])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
        }
        Some("finish-marker") => {
            std::thread::sleep(Duration::from_millis(150));
            std::fs::write(args.next().unwrap(), b"finished").unwrap();
        }
        _ => std::process::exit(90),
    }
}
