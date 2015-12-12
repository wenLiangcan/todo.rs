#![feature(plugin)]

#![plugin(clippy)]

use std::str::FromStr;
use std::path::Path;
use std::io::prelude::*;
use std::io::BufReader;
use std::io;
use std::fs::OpenOptions;
use std::fmt;
use std::env;
use std::error::Error;

extern crate regex;
use regex::Regex;

extern crate ansi_term;
use ansi_term::Colour::*;
use ansi_term::Style;

#[macro_use]
extern crate clap;
use clap::{App, Arg, SubCommand, AppSettings};

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
        match *self {
            Task::DoneTask(ref task_data) => write!(f, "- [x] {}", task_data),
            Task::TodoTask(ref task_data) => write!(f, "- [ ] {}", task_data),
        }
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Task::DoneTask(ref task_data) => write!(f, "{} {}", Green.paint("✓"), task_data),
            Task::TodoTask(ref task_data) => write!(f, "{} {}", Red.paint("✖"), task_data),
        }
    }
}

impl Task {
    fn new(note: &str) -> Self {
        Task::TodoTask(
            TaskData {
                note: note.to_owned(),
            }
        )
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
            Some(cap) => {
                let task_data = TaskData{
                    note: cap.at(2).unwrap().to_owned()
                };
                Ok(
                    match cap.at(1) {
                        Some("x") => Task::DoneTask(task_data),
                        Some(" ") => Task::TodoTask(task_data),
                        _ => panic!("Unkown status"),
                    }
                )
            },
            None => Err(TaskParseError),
        }
    }
}

fn filter_print_lines<I, F>(iter: I, f: F)
    where I: Iterator,
          I::Item: fmt::Display,
          F: Fn(&I::Item) -> bool
{

    for (i, t) in iter.enumerate().filter(|pair| {
        match *pair {
            (_, ref t) => f(t)
        }
    }) {
        println!(" {} {}",
                 Style::default().dimmed().paint(&format!("{}.", i+1)[..]), t);
    }
}

fn check_vec_bounds<T>(v: &[T], index: usize) -> bool {
    v.get(index).map_or(false, |_| true)
}

struct TodoList<'p> {
    path: &'p Path,
    list: Vec<Task>,
}

impl<'p> TodoList<'p> {
    fn load(path: &'p Path) -> Result<Self, io::Error> {
        let file = OpenOptions::new()
                    .read(true)
                    .create(true)
                    .open(path);

        file.map(|f| {
            let reader = BufReader::new(&f);
            let list: Vec<Task> = reader.lines().map(|l| {
                match l {
                    Ok(s) => s.parse::<Task>().unwrap(),
                    Err(e) => panic!("{}", Error::description(&e)),
                }}).collect();
            TodoList {
                path: path,
                list: list,
            }
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

    fn add(&mut self, note: &str) {
        let task = Task::new(note);
        self.list.push(task);
        self.save();
    }

    fn check(&mut self, index: usize) {
        let i = index - 1;
        if check_vec_bounds(&self.list, i) {
            let t = self.list.remove(i);
            self.list.insert(i, t.check());
            self.save();
        }
    }

    fn undo(&mut self, index: usize) {
        let i = index - 1;
        if check_vec_bounds(&self.list, i) {
            let t = self.list.remove(i);
            self.list.insert(i, t.undo());
            self.save();
        }
    }

    fn remove(&mut self, index: usize) {
        let i = index - 1;
        if check_vec_bounds(&self.list, i) {
            self.list.remove(i);
            self.save();
        }
    }

    fn cleanup(&mut self) {
        self.list.retain(|task| match *task {
            Task::TodoTask(_) => true,
            _ => false,
        });
        self.save();
    }

    fn clear(&mut self) {
        self.list.clear();
        self.save();
    }

    fn print_unchecked(&self) {
        filter_print_lines(self.list.iter(),
                           |&t| {
                               match *t {
                                   Task::TodoTask(_) => true,
                                   _ => false,
                               }
                           });
    }

    fn print_all(&self) {
        filter_print_lines(self.list.iter(), |_| true);
    }
}

#[allow(str_to_string)]  // omit the warning of `str_to_string` caused by `clap`
fn main() {
    let args = App::new("todo")
                        .version("0.2.0")
                        .about("CLI Todo-List Tool")
                        .settings(&[AppSettings::SubcommandsNegateReqs,
                                    AppSettings::VersionlessSubcommands])
                        .arg(Arg::with_name("task")
                             .required(true)
                             .index(1)
                             .help("Add a new task"))
                        .subcommand(SubCommand::with_name("ls")
                                    .about("List unchecked tasks")
                                    .arg(Arg::with_name("list all")
                                         .long("all")
                                         .help("List all tasks")))
                        .subcommand(SubCommand::with_name("remove")
                                    .about("Remove a task by index")
                                    .arg(Arg::with_name("index")
                                         .required(true)))
                        .subcommand(SubCommand::with_name("check")
                                    .about("Check a task by index")
                                    .arg(Arg::with_name("index")
                                         .required(true)))
                        .subcommand(SubCommand::with_name("undo")
                                    .about("Undo a task by index")
                                    .arg(Arg::with_name("index")
                                         .required(true)))
                        .subcommand(SubCommand::with_name("cleanup")
                                    .about("Clear checked tasks"))
                        .subcommand(SubCommand::with_name("clear")
                                    .about("Clear all tasks"))
                    .get_matches();

    let pbf = &mut env::home_dir().unwrap();
    pbf.push("todo.txt");
    let path = pbf.as_path();
    let mut todo_list = TodoList::load(path).unwrap();

    if let Some(task) = args.value_of("task") {
        todo_list.add(task);
    }

    match args.subcommand() {
        ("ls", Some(matches)) => if matches.is_present("list all") {
            todo_list.print_all();
            return
        },
        ("cleanup", Some(_)) => todo_list.cleanup(),
        ("clear", Some(_)) => todo_list.clear(),
        (action, Some(matches)) => {
            let i = value_t_or_exit!(matches.value_of("index"), usize);
            match action {
                "remove" => todo_list.remove(i),
                "check" => todo_list.check(i),
                "undo" => todo_list.undo(i),
                _ => {}
            }
        }
        _ => {}
    };

    todo_list.print_unchecked();
}
