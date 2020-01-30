/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;

use std::path::PathBuf;
use std::process::Command;
use url::Url;

mod blink;
mod gecko;
mod webkit;

pub(crate) trait CrashtestProvider: std::iter::Iterator<Item = Url> + Send {}

pub(crate) enum CrashtestResult {
    Ok,
    Skipped,
    Timeout { stdout: String, stderr: String },
    Crashed { stdout: String, stderr: String },
}

pub(crate) trait CrashtestRunner: Sync {
    fn run(&self, url: &Url) -> CrashtestResult;
}

fn main() {
    let matches = app_from_crate!()
        .args_from_usage(
            "--source <engine> 'Engine to run crashtests for'
             --target <engine> 'Engine to run crashtests on'
             <source-path> 'Path to the source engine's source directory'
             <target-path> 'Path to the target engines' object directory'",
        )
        .get_matches();

    env_logger::init();

    let source_path = PathBuf::from(matches.value_of("source-path").unwrap());
    let target_path = PathBuf::from(matches.value_of("target-path").unwrap());

    let crashtests = match matches.value_of("source").unwrap() {
        "gecko" => {
            Box::new(gecko::CrashtestProvider::new(source_path)) as Box<dyn CrashtestProvider>
        }
        "blink" => {
            Box::new(blink::CrashtestProvider::new(source_path)) as Box<dyn CrashtestProvider>
        }
        // FIXME(emilio): implement webkit stuff.
        engine => panic!("Unimplemented source engine {}, use: gecko, blink", engine),
    };

    let consumer = match matches.value_of("target").unwrap() {
        "gecko" => Box::new(gecko::CrashtestRunner::new(target_path)) as Box<dyn CrashtestRunner>,
        "blink" => Box::new(blink::CrashtestRunner::new(target_path)) as Box<dyn CrashtestRunner>,
        "webkit" => Box::new(webkit::CrashtestRunner::new(target_path)) as Box<dyn CrashtestRunner>,
        engine => panic!("Unimplemented target engine {}, use: gecko, blink, webkit", engine),
    };

    const LIST: bool = false;

    rayon::scope(|scope| {
        let consumer = &consumer;
        for c in crashtests {
            if LIST {
                println!("{}", c);
                continue;
            }
            scope.spawn(move |_| {
                let c = c;
                match consumer.run(&c) {
                    CrashtestResult::Ok => println!("OK: {}", c),
                    CrashtestResult::Timeout { .. } => println!("TIMEOUT: {}", c),
                    CrashtestResult::Skipped => println!("SKIP: {}", c),
                    CrashtestResult::Crashed { .. } => println!("CRASHED: {}", c),
                }
            })
        }
    });
}

fn run_crashtest_command(mut command: Command) -> CrashtestResult {
    use std::io::Read;
    use std::process::Stdio;
    use std::time::Duration;
    use wait_timeout::ChildExt;

    command.stderr(Stdio::piped()).stdout(Stdio::piped());

    let mut child = command.spawn().expect("Couldn't run");
    let status = match child.wait_timeout(Duration::from_secs(20)).expect("Couldn't wait for child") {
        Some(status) if status.success() => {
            return CrashtestResult::Ok;
        }
        other => other,
    };

    if status.is_none() {
        child.kill().expect("Couldn't kill after timeout");
    }

    let mut stderr = String::new();
    let mut stdout = String::new();
    child.stderr.take().unwrap().read_to_string(&mut stderr).expect("Non-utf8 stderr?");
    child.stdout.take().unwrap().read_to_string(&mut stdout).expect("Non-utf8 stdout?");
    if status.is_none() {
        CrashtestResult::Timeout { stdout, stderr }
    } else {
        CrashtestResult::Crashed { stdout, stderr }
    }
}
