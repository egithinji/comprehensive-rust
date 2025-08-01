// Copyright 2023 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! This binary allows us to execute tasks within the project by running
//! `cargo xtask <task>`. It can thus be used as a task automation tool.
//! For example instead of repeatedly running `cargo install` from the CLI
//! to install all the necessary tools for the project we can just run
//! `cargo xtask install-tools` and the logic defined here will install
//! the tools.

use anyhow::{Ok, Result, anyhow};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::{env, process::Command};

fn main() -> Result<()> {
    if let Err(e) = execute_task() {
        eprintln!("{e}");
        std::process::exit(-1);
    }
    Ok(())
}

#[derive(Parser)]
#[command(
    about = "Binary for executing tasks within the Comprehensive Rust project"
)]
struct Cli {
    /// The task to execute
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Installs the tools the project depends on.
    InstallTools,
    /// Runs the web driver tests in the tests directory.
    WebTests,
    /// Tests all included Rust snippets.
    RustTests,
    /// Starts a web server with the course.
    Serve {
        /// ISO 639 language code (e.g. da for the Danish translation).
        #[arg(short, long)]
        language: Option<String>,

        /// Directory to place the build. If not provided, defaults to the book/ directory (or the book/xx directory if a language is provided).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Create a static version of the course.
    Build {
        /// ISO 639 language code (e.g. da for the Danish translation).
        #[arg(short, long)]
        language: Option<String>,

        /// Directory to place the build. If not provided, defaults to the book/ directory (or the book/xx directory if a language is provided).
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn execute_task() -> Result<()> {
    let cli = Cli::parse();
    match cli.task {
        Task::InstallTools => install_tools()?,
        Task::WebTests => run_web_tests()?,
        Task::RustTests => run_rust_tests()?,
        Task::Serve { language, output } => start_web_server(language, output)?,
        Task::Build { language, output } => build(language, output)?,
    }
    Ok(())
}

fn install_tools() -> Result<()> {
    println!("Installing project tools...");
    let path_to_mdbook_exerciser =
        Path::new(env!("CARGO_WORKSPACE_DIR")).join("mdbook-exerciser");
    let path_to_mdbook_course =
        Path::new(env!("CARGO_WORKSPACE_DIR")).join("mdbook-course");

    let install_args = vec![
        // The --locked flag is important for reproducible builds. It also
        // avoids breakage due to skews between mdbook and mdbook-svgbob.
        vec!["mdbook", "--locked", "--version", "0.4.48"],
        vec!["mdbook-svgbob", "--locked", "--version", "0.2.2"],
        vec!["mdbook-pandoc", "--locked", "--version", "0.10.4"],
        vec!["mdbook-i18n-helpers", "--locked", "--version", "0.3.6"],
        vec!["i18n-report", "--locked", "--version", "0.2.0"],
        vec!["mdbook-linkcheck2", "--locked", "--version", "0.9.1"],
        // Mdbook-exerciser and mdbook-course are located in this repository.
        // To make it possible to install them from any directory we need to
        // specify their path from the workspace root.
        vec!["--path", path_to_mdbook_exerciser.to_str().unwrap(), "--locked"],
        vec!["--path", path_to_mdbook_course.to_str().unwrap(), "--locked"],
    ];

    for args in &install_args {
        let status = Command::new(env!("CARGO"))
            .arg("install")
            .args(args)
            .status()
            .expect("Failed to execute cargo install");

        if !status.success() {
            let error_message = format!(
                "Command 'cargo install {}' exited with status code: {}",
                args.join(" "),
                status.code().unwrap()
            );
            return Err(anyhow!(error_message));
        }
    }

    Ok(())
}

fn run_web_tests() -> Result<()> {
    println!("Running web tests...");

    let path_to_tests_dir = Path::new(env!("CARGO_WORKSPACE_DIR")).join("tests");

    let status = Command::new("npm")
        .current_dir(path_to_tests_dir.to_str().unwrap())
        .arg("test")
        .status()
        .expect("Failed to execute npm test");

    if !status.success() {
        let error_message = format!(
            "Command 'cargo xtask web-tests' exited with status code: {}",
            status.code().unwrap()
        );
        return Err(anyhow!(error_message));
    }

    Ok(())
}

fn run_rust_tests() -> Result<()> {
    println!("Running rust tests...");
    let path_to_workspace_root = Path::new(env!("CARGO_WORKSPACE_DIR"));

    let status = Command::new("mdbook")
        .current_dir(path_to_workspace_root.to_str().unwrap())
        .arg("test")
        .status()
        .expect("Failed to execute mdbook test");

    if !status.success() {
        let error_message = format!(
            "Command 'cargo xtask rust-tests' exited with status code: {}",
            status.code().unwrap()
        );
        return Err(anyhow!(error_message));
    }

    Ok(())
}

fn start_web_server(
    language: Option<String>,
    output_arg: Option<PathBuf>,
) -> Result<()> {
    println!("Starting web server ...");
    let path_to_workspace_root = Path::new(env!("CARGO_WORKSPACE_DIR"));

    let mut command = Command::new("mdbook");
    command.current_dir(path_to_workspace_root.to_str().unwrap());
    command.arg("serve");

    if let Some(language) = &language {
        println!("Language: {}", &language);
        command.env("MDBOOK_BOOK__LANGUAGE", &language);
    }

    command.arg("-d");
    command.arg(get_output_dir(language, output_arg));

    let status = command.status().expect("Failed to execute mdbook serve");

    if !status.success() {
        let error_message = format!(
            "Command 'cargo xtask serve' exited with status code: {}",
            status.code().unwrap()
        );
        return Err(anyhow!(error_message));
    }
    Ok(())
}

fn build(language: Option<String>, output_arg: Option<PathBuf>) -> Result<()> {
    println!("Building course...");
    let path_to_workspace_root = Path::new(env!("CARGO_WORKSPACE_DIR"));

    let mut command = Command::new("mdbook");
    command.current_dir(path_to_workspace_root.to_str().unwrap());
    command.arg("build");

    if let Some(language) = &language {
        println!("Language: {}", &language);
        command.env("MDBOOK_BOOK__LANGUAGE", language);
    }

    command.arg("-d");
    command.arg(get_output_dir(language, output_arg));

    let status = command.status().expect("Failed to execute mdbook build");

    if !status.success() {
        let error_message = format!(
            "Command 'cargo xtask build' exited with status code: {}",
            status.code().unwrap()
        );
        return Err(anyhow!(error_message));
    }
    Ok(())
}

fn get_output_dir(language: Option<String>, output_arg: Option<PathBuf>) -> PathBuf {
    // If the 'output' arg is specified by the caller, use that, otherwise output to the 'book/' directory
    // (or the 'book/xx' directory if a language was specified).
    if let Some(d) = output_arg {
        d
    } else {
        Path::new("book").join(language.unwrap_or("".to_string()))
    }
}
