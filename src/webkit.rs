/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::path::PathBuf;
use url::Url;
use std::process::Command;

pub struct CrashtestRunner {
    test_runner: PathBuf,
    plugin_path: PathBuf,
    test_runner_injected_bundle: PathBuf,
    dependencies_path: PathBuf,
}

impl CrashtestRunner {
    pub fn new(build_dir: PathBuf) -> Self {
        let build_dir = build_dir.canonicalize().unwrap();
        Self {
            test_runner: build_dir.join("Debug").join("bin").join("WebKitTestRunner"),
            plugin_path: build_dir.join("Debug").join("lib").join("plugins"),
            test_runner_injected_bundle: build_dir.join("Debug").join("lib").join("libTestRunnerInjectedBundle.so"),
            dependencies_path: build_dir.join("DependenciesGTK").join("Root").join("lib"),
        }
    }
}

impl super::CrashtestRunner for CrashtestRunner {
    fn run(&self, url: &Url) -> super::CrashtestResult {
        if url.scheme() != "file" {
            return super::CrashtestResult::Skipped;
        }

        let mut command = Command::new(&self.test_runner);

        command
            .env("LD_LIBRARY_PATH", &self.dependencies_path)
            .env("TEST_RUNNER_TEST_PLUGIN_PATH", &self.plugin_path)
            .env("TEST_RUNNER_INJECTED_BUNDLE_FILENAME", &self.test_runner_injected_bundle)
            .arg(url.to_string());

        super::run_crashtest_command(command)
    }
}
