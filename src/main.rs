use std::{
    error::Error,
    fs::{self},
    path::PathBuf,
    process::Command,
};

use clap::Parser;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use similar::{ChangeTag, DiffableStr, TextDiff};

#[derive(Debug, clap::Parser)]
#[clap(author, version, about)]
pub struct Args {
    #[clap(short, long, required = true)]
    pub file: PathBuf,

    #[clap(short, long)]
    pub clear: bool,
}

fn main() {
    let args = Args::parse();
    if let Err(error) = watch(&args.file) {
        println!("Error: {error:?}");
    }
}

fn watch(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let mut versions: Vec<String> = Vec::new();
    let zero = fs::read_to_string(path)?;
    versions.push(zero);
    let (tx, rx) = std::sync::mpsc::channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;

    while let Ok(res) = rx.try_recv() {
        match res {
            Ok(event) => match event.kind {
                notify::EventKind::Modify(event) => {
                    match event {
                        notify::event::ModifyKind::Data(_) => {
                            let prev = versions.last().unwrap().clone();
                            let contents = fs::read_to_string(path)?;
                            if prev != contents {
                                versions.push(contents);
                                let len = versions.len();
                                // print_diff(&versions[len - 2], &versions[len - 1]);
                                print_diff_delta(&versions[len - 2], &versions[len - 1], false);
                            }
                        }
                        _ => {}
                    }
                }
                notify::EventKind::Any => {}
                notify::EventKind::Access(_) => {}
                notify::EventKind::Create(_) => {}
                notify::EventKind::Remove(_) => {}
                notify::EventKind::Other => {}
            },
            Err(error) => println!("Error: {error:?}"),
        }
    }

    Ok(())
}

fn print_diff(old: &str, new: &str) {
    println!("OLD: \n{old}\n NEW: \n{new}");

    let diff = TextDiff::from_lines(old, new);

    for op in diff.ops() {
        for change in diff.iter_changes(op) {
            let (sign, style) = match change.tag() {
                ChangeTag::Delete => ("-", console::Style::new().red()),
                ChangeTag::Insert => ("+", console::Style::new().green()),
                ChangeTag::Equal => (" ", console::Style::new()),
            };
            print!("{}{}", style.apply_to(sign).bold(), style.apply_to(change));
        }
    }
}

fn print_diff_delta(old: &str, new: &str, clear: bool) {
    let old_file = tempfile::NamedTempFile::new().unwrap();
    let new_file = tempfile::NamedTempFile::new().unwrap();
    let _ = std::fs::write(old_file.path(), old);
    let _ = std::fs::write(new_file.path(), new);

    // clear screen
    if clear {
        print!("\x1B[2J\x1B[1;1H");
    } else {
        println!("----------------------------------------------------------------");
    }

    let output = Command::new("delta")
        .arg(old_file.path().to_string_lossy().as_str().unwrap())
        .arg(new_file.path().to_string_lossy().as_str().unwrap())
        .output()
        .unwrap();
    print!("{}", std::str::from_utf8(output.stdout.as_slice()).unwrap());
}
