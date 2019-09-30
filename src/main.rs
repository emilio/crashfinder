/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;

use std::path::PathBuf;
use url::Url;

mod gecko;
mod blink;

pub(crate) trait CrashtestProvider : std::iter::IntoIterator<Item = Url> {}

pub(crate) enum CrashtestResult {
    Ok,
    Skipped,
    Timeout { stdout: String, stderr: String },
    Crashed { stdout: String, stderr: String },
}

pub(crate) trait CrashtestRunner {
    fn run(&self, url: &Url) -> CrashtestResult;
}

fn main() {
    let matches = app_from_crate!()
        .args_from_usage(
            "<gecko-tree> 'Path to gecko'
             <content-shell> 'Path to Chromium\'s content_shell'"
        )
        .get_matches();

    env_logger::init();

    let gecko_tree = PathBuf::from(matches.value_of("gecko-tree").unwrap());
    let content_shell = PathBuf::from(matches.value_of("content-shell").unwrap());

    let crashtests = gecko::CrashtestProvider::new(gecko_tree);
    let consumer = blink::CrashtestRunner::new(content_shell);

    rayon::scope(|scope| {
        let consumer = &consumer;
        for c in crashtests {
            scope.spawn(move |_| {
                let c = c;
                match consumer.run(&c) {
                    CrashtestResult::Ok => println!("OK: {}", c),
                    CrashtestResult::Timeout { .. }=> println!("TIMEOUT: {}", c),
                    CrashtestResult::Skipped => println!("SKIP: {}", c),
                    CrashtestResult::Crashed { .. } => println!("CRASHED: {}", c),
                }
            })
        }
    });
}
