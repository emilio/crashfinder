/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::path::PathBuf;
use std::fs::File;
use std::io::{BufRead, BufReader, Lines};
use url::Url;
use std::process::Command;

pub struct CrashtestList {
    /// Path to crashtests.list
    url: Url,
    lines: Lines<BufReader<File>>,
    children: Vec<CrashtestList>,
}


impl CrashtestList {
    fn new(list_url: Url) -> Self {
        let list_path = list_url.to_file_path().expect("manifest should be local");
        let file = match File::open(&list_path) {
            Ok(f) => f,
            Err(e) => panic!("Couldn't read crashtest.list file: {} {:?}", list_path.display(), e),
        };

        Self {
            url: list_url,
            lines: BufReader::new(file).lines(),
            children: vec![],
        }
    }
}

impl Iterator for CrashtestList {
    type Item = Url;

    fn next(&mut self) -> Option<Url> {
        loop {
            while !self.children.is_empty() {
                if let Some(path) = self.children.last_mut().unwrap().next() {
                    return Some(path);
                }
                self.children.pop();
            }

            let mut line = self.lines.next()?.expect("Couldn't read a line from manifest");
            // Strip comments.
            if let Some(pos) = line.find('#') {
                line.truncate(pos);
            }

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut parts = line.split(" ");
            let command = match parts.next() {
                Some(p) => p,
                None => {
                    warn!("Ignoring manifest line {}", line);
                    continue;
                }
            };

            let is_include = match command {
                "load" => false,
                "include" => true,
                _ => {
                    warn!("Ignoring manifest line {}", line);
                    continue;
                }
            };

            let url = match parts.next() {
                Some(f) => self.url.join(f).expect("Invalid url in manifest"),
                _ => {
                    error!("Ignoring manifest line with invalid load / include {}", line);
                    continue;
                }
            };

            if !is_include {
                return Some(url);
            }

            self.children.push(CrashtestList::new(url));
        }
    }
}

pub struct CrashtestProvider {
    // gecko_root: PathBuf,
    root_list: CrashtestList,
}

impl CrashtestProvider {
    pub fn new(gecko_root: PathBuf) -> Self {
        let crashtests_list = gecko_root.join("testing").join("crashtest").join("crashtests.list");
        let crashtests_list = crashtests_list.canonicalize().unwrap();
        Self {
            // gecko_root: gecko_root.clone(),
            root_list: CrashtestList::new(
                Url::from_file_path(crashtests_list).unwrap()
            ),
        }
    }
}

impl Iterator for CrashtestProvider {
    type Item = Url;

    fn next(&mut self) -> Option<Url> {
        self.root_list.next()
    }
}

impl super::CrashtestProvider for CrashtestProvider {}

pub struct CrashtestRunner {
    objdir: PathBuf,
}

impl CrashtestRunner {
    pub fn new(objdir: PathBuf) -> Self {
        Self {
            objdir: objdir.canonicalize().unwrap(),
        }
    }
}

impl super::CrashtestRunner for CrashtestRunner {
    fn run(&self, url: &Url) -> super::CrashtestResult {
        if url.scheme() != "file" {
            return super::CrashtestResult::Skipped;
        }

        let tempdir = tempdir::TempDir::new("firefox-crashtest")
            .expect("couldn't create temporary profile directory");

        let firefox = self.objdir.join("dist").join("bin").join("firefox");

        let mut command = Command::new(&firefox);

        command.env("MOZ_GDB_SLEEP", "0");
        command.env("MOZ_HEADLESS", "1");

        command
            .arg("-layoutdebug")
            .arg(url.to_string())
            .arg("-autoclose")
            .arg("-no-remote")
            .arg("-profile")
            .arg(tempdir.path());

        super::run_crashtest_command(command)
    }
}
