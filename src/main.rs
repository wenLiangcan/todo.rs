#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

use std::fmt;
use std::fs::OpenOptions;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use std::str::FromStr;

use regex::Regex;

use ansi_term::Colour::*;
use ansi_term::Style;

#[macro_use]
extern crate clap;
use clap::{App, AppSettings, Arg, SubCommand};

struct TaskData {
    note: String,
}

impl fmt::Display for TaskData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.note)
    }
}

enum Task {
    DoneTask(TaskData),
    TodoTask(TaskData),
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Task::DoneTask(task_data) => write!(f, "- [x] {}", task_data),
            Task::TodoTask(task_data) => write!(f, "- [ ] {}", task_data),
        }
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Task::DoneTask(task_data) => write!(f, "{} {}", Green.paint("✓"), task_data),
            Task::TodoTask(task_data) => write!(f, "{} {}", Red.paint("✖"), task_data),
        }
    }
}

impl Task {
    fn new(note: &str) -> Self {
        Task::TodoTask(TaskData {
            note: note.to_owned(),
        })
    }

    fn check(self) -> Self {
        match self {
            Task::TodoTask(task_data) => Task::DoneTask(task_data),
            Task::DoneTask(_) => self,
        }
    }

    fn undo(self) -> Self {
        match self {
            Task::DoneTask(task_data) => Task::TodoTask(task_data),
            Task::TodoTask(_) => self,
        }
    }
}

#[derive(Debug)]
struct TaskParseError;

impl FromStr for Task {
    type Err = TaskParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r"^- \[([\sx])\] (.*)$").unwrap();
        match re.captures(s) {
            Some(cap) => cap
                .get(2)
                .map(|n| TaskData {
                    note: n.as_str().to_string(),
                })
                .and_then(|task_data| match cap.get(1).map(|m| m.as_str()) {
                    Some("x") => Some(Task::DoneTask(task_data)),
                    Some(" ") => Some(Task::TodoTask(task_data)),
                    _ => None,
                })
                .ok_or(TaskParseError),
            None => Err(TaskParseError),
        }
    }
}

fn filter_print_lines<I, F>(iter: I, f: F)
where
    I: Iterator,
    I::Item: fmt::Display,
    F: Fn(&I::Item) -> bool,
{
    for (i, t) in iter.enumerate().filter(|pair| match pair {
        (_, t) => f(t),
    }) {
        println!(
            " {} {}",
            Style::default().dimmed().paint(&format!("{}.", i + 1)[..]),
            t
        );
    }
}

fn vec_try_remove<T>(v: &mut Vec<T>, index: usize) -> Option<T> {
    if index < v.len() {
        Some(v.remove(index))
    } else {
        None
    }
}

struct TodoList<'p> {
    path: &'p Path,
    list: Vec<Task>,
}

impl<'p> TodoList<'p> {
    fn load(path: &'p Path) -> Result<Self, io::Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        let reader = BufReader::new(file);
        let list: Vec<Task> = reader
            .lines()
            .enumerate()
            .map(|(i, l)| match l {
                Ok(s) => s
                    .parse::<Task>()
                    .expect(&format!("Failed to parse line {}", i)),
                Err(e) => panic!("{:#?}", e),
            })
            .collect();
        Ok(TodoList {
            path: path,
            list: list,
        })
    }

    fn save(&self) {
        let mut file = OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .open(self.path)
            .unwrap();

        for l in &self.list {
            writeln!(file, "{:?}", l).unwrap();
        }
    }

    fn modify<F>(&mut self, action: F)
    where
        F: FnOnce(&mut Vec<Task>),
    {
        action(&mut self.list);
        self.save();
    }

    fn add(&mut self, note: &str) {
        self.modify(|l| {
            let task = Task::new(note);
            l.push(task);
        })
    }

    fn check(&mut self, index: usize) {
        let i = index - 1;
        if let Some(t) = vec_try_remove(&mut self.list, i) {
            self.modify(|l| {
                l.insert(i, t.check());
            })
        }
    }

    fn undo(&mut self, index: usize) {
        let i = index - 1;
        if let Some(t) = vec_try_remove(&mut self.list, i) {
            self.modify(|l| {
                l.insert(i, t.undo());
            })
        }
    }

    fn remove(&mut self, index: usize) {
        let i = index - 1;
        if let Some(_) = vec_try_remove(&mut self.list, i) {
            self.save();
        }
    }

    fn cleanup(&mut self) {
        self.modify(|l| {
            l.retain(|task| match task {
                Task::TodoTask(_) => true,
                _ => false,
            });
        })
    }

    fn clear(&mut self) {
        self.modify(|l| {
            l.clear();
        })
    }

    fn print_unchecked(&self) {
        filter_print_lines(self.list.iter(), |t| match t {
            Task::TodoTask(_) => true,
            _ => false,
        });
    }

    fn print_all(&self) {
        filter_print_lines(self.list.iter(), |_| true);
    }
}



fn main() {
    let args = App::new("todo")
        .version("0.2.0")
        .about("CLI Todo-List Tool")
        .settings(&[
            AppSettings::SubcommandsNegateReqs,
            AppSettings::VersionlessSubcommands,
        ])
        .arg(
            Arg::with_name("task")
                .required(true)
                .index(1)
                .help("Add a new task"),
        )
        .subcommand(
            SubCommand::with_name("ls")
                .about("List unchecked tasks")
                .arg(
                    Arg::with_name("list all")
                        .long("all")
                        .help("List all tasks"),
                ),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .about("Remove a task by index")
                .arg(Arg::with_name("index").required(true)),
        )
        .subcommand(
            SubCommand::with_name("check")
                .about("Check a task by index")
                .arg(Arg::with_name("index").required(true)),
        )
        .subcommand(
            SubCommand::with_name("undo")
                .about("Undo a task by index")
                .arg(Arg::with_name("index").required(true)),
        )
        .subcommand(SubCommand::with_name("cleanup").about("Clear checked tasks"))
        .subcommand(SubCommand::with_name("clear").about("Clear all tasks"))
        .get_matches();

    let path = dirs::home_dir().unwrap().join("todo.txt");
    let mut todo_list = TodoList::load(&path).unwrap();

    if let Some(task) = args.value_of("task") {
        todo_list.add(task);
    }

    match args.subcommand() {
        ("ls", Some(matches)) => {
            if matches.is_present("list all") {
                todo_list.print_all();
                return;
            }
        }
        ("cleanup", Some(_)) => todo_list.cleanup(),
        ("clear", Some(_)) => todo_list.clear(),
        (action, Some(matches)) => {
            let i = value_t_or_exit!(matches.value_of("index"), usize);
            match action {
                "remove" => todo_list.remove(i),
                "check" => todo_list.check(i),
                "undo" => todo_list.undo(i),
                _ => (),
            }
        }
        _ => (),
    };

    todo_list.print_unchecked();
}
