#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

#[macro_use]
extern crate clap;
use clap::{App, AppSettings, Arg, SubCommand};

use todo::*;

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
