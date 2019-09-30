/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::path::PathBuf;
use url::Url;
use std::process::{Command, Stdio};
use std::io::Read;
use std::time::Duration;
use wait_timeout::ChildExt;

pub struct CrashtestRunner {
    content_shell: PathBuf,
}

impl CrashtestRunner {
    pub fn new(content_shell: PathBuf) -> Self {
        Self {
            content_shell: content_shell.canonicalize().unwrap(),
        }
    }
}

impl super::CrashtestRunner for CrashtestRunner {
    fn run(&self, url: &Url) -> super::CrashtestResult {
        if url.scheme() != "file" {
            return super::CrashtestResult::Skipped;
        }

        let mut command = Command::new(&self.content_shell);

        command
            .arg("--run-web-tests")
            .arg("--single-process")
            .arg("--enable-experimental-web-platform-features")
            .arg(url.to_string())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped());

        let mut child = command.spawn().expect("Couldn't run content_shell");
        let status = match child.wait_timeout(Duration::from_secs(20)).expect("Couldn't wait for child") {
            Some(status) if status.success() => {
                return super::CrashtestResult::Ok;
            }
            other => other,
        };

        if status.is_none() {
            child.kill().expect("Couldn't kill content_shell after timeout");
        }

        let mut stderr = String::new();
        let mut stdout = String::new();
        child.stderr.take().unwrap().read_to_string(&mut stderr).expect("Non-utf8 stderr?");
        child.stdout.take().unwrap().read_to_string(&mut stdout).expect("Non-utf8 stdout?");
        if status.is_none() {
            super::CrashtestResult::Timeout { stdout, stderr }
        } else {
            super::CrashtestResult::Crashed { stdout, stderr }
        }
    }
}
