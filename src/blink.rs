/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::path::PathBuf;
use std::process::Command;
use url::Url;
use walkdir::WalkDir;

pub struct CrashtestRunner {
    content_shell: PathBuf,
}

impl CrashtestRunner {
    pub fn new(out_path: PathBuf) -> Self {
        Self {
            content_shell: out_path.canonicalize().unwrap().join("content_shell"),
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
            .arg(url.to_string());

        super::run_crashtest_command(command)
    }
}

pub struct CrashtestProvider {
    walkdir: walkdir::IntoIter,
}

impl CrashtestProvider {
    pub fn new(chromium_src: PathBuf) -> Self {
        let chromium_src = chromium_src.canonicalize().unwrap();
        let web_tests = chromium_src.join("third_party").join("blink").join("web_tests");
        let walkdir = WalkDir::new(web_tests).follow_links(true).into_iter();

        Self { walkdir }
    }
}

impl Iterator for CrashtestProvider {
    type Item = Url;

    fn next(&mut self) -> Option<Url> {
        loop {
            let entry = self.walkdir.next()?.unwrap();
            if entry.file_type().is_file() {
                if entry.depth() == 0 {
                    continue;
                }
                if let Some(stem) = entry.path().file_stem() {
                    let stem = stem.to_string_lossy();
                    if stem.ends_with("-expected") || stem == "README" || stem == "OWNERS" {
                        continue;
                    }
                    // FIXME(emilio): Maybe too overkill? Otherwise we get ~40k
                    // tests rather than ~2k.
                    if !stem.contains("crash") {
                        continue;
                    }
                }
                return Some(Url::from_file_path(entry.into_path()).unwrap());
            }
            assert!(entry.file_type().is_dir(), "We follow symlinks so...");
            if entry.file_name().to_str().map_or(false, |s| s == "resources" || s == "external" || s == "third_party") {
                self.walkdir.skip_current_dir();
                continue;
            }
        }
    }
}

impl super::CrashtestProvider for CrashtestProvider {}
