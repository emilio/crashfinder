/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::path::PathBuf;
use std::fs::File;
use std::io::{BufRead, BufReader, Lines};
use url::Url;

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

impl IntoIterator for CrashtestProvider {
    type Item = Url;
    type IntoIter = CrashtestList;

    fn into_iter(self) -> Self::IntoIter {
        self.root_list
    }
}

impl super::CrashtestProvider for CrashtestProvider {}
