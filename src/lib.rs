use std::ffi::*;
use std::os::raw::*;

use std::io::{BufRead, BufReader};
use std::os::unix::prelude::*;
use std::os::unix::net::{UnixStream, UnixListener};

use std::collections::HashMap;

use std::sync::mpsc;
use std::sync::Mutex;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref SENDER: Mutex<Option<mpsc::Sender<Message>>> = Mutex::new(None);
}



/// Command struct holding a command to execute
#[repr(C)]
pub struct Command_r {
    name: *const c_char,
    func: fn(arg: *const Arg_r),
    arg: Arg_r,
}

#[derive(Send)]
struct Command {
    name: String,
    func: fn(arg: *const Arg_r),
    arg: Box<Arg_r>,
}

impl Command {
    unsafe fn from_ptr(commands: *const Command_r, len: c_int) -> HashMap<String, Self> {
        println!("parsing {} commands", len);
        let mut ret = HashMap::new();
        let commands = std::slice::from_raw_parts(commands, len as usize);
        for v in commands.iter() {
            let name = CStr::from_ptr(v.name).to_string_lossy().to_string();
            println!("add {}", name);
            let func = v.func;
            let arg = Box::new(v.arg);

            ret.insert(name.clone(), Self{
                name,
                func,
                arg,
            });
        }

        ret
    }

    fn run(&self) {
        let func: fn(arg: *const Arg_r) = self.func;
        func(self.arg.as_ref());
    }
}

/// arguments for funtions described with `Command_r`
#[repr(C)]
#[derive(Copy, Clone)]
union Arg_r {
    i: c_int,
    ui: c_uint,
    f: c_float,
    v: *const c_void,
}



fn parse_vars(input: &str) -> String {
    if input.starts_with('$') {
        let pieces: Vec<&str> = input.split('/').collect();
        let mut pieces: Vec<String> = pieces.iter().map(|s| String::from(*s)).collect();
        for i in 0..pieces.len() {
            if pieces[i].starts_with('$') {
                let name = pieces[i].trim_matches('$');
                let name = name.trim();
                let value = std::env::var(name).unwrap_or(String::from(""));
                pieces[i] = value;
            }
        }
        return pieces.join("/");
    }
    input.to_string()
}

#[no_mangle]
pub extern "C" fn init_rwm(path: *const c_char, commands: *const Command_r, len: c_int) -> c_int {
    let (tx, rx): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel();

    let mut sender = SENDER.lock().unwrap();
    *sender = Some(tx);

    let path = unsafe { CStr::from_ptr(path).to_string_lossy() };
    let path = parse_vars(&path);

    // parse commands
    /* let commands = unsafe { std::slice::from_raw_parts(commands, len as usize)};
    let name = commands[1].name;
    let name = unsafe { CStr::from_ptr(name).to_string_lossy()};
    let function: fn(arg: *const Arg_r) = commands[1].func as fn(arg: *const Arg_r);
    function(&commands[1].arg); */
    let commands = unsafe { Command::from_ptr(commands, len)};

    std::thread::spawn(move || {
        std::fs::remove_file(std::path::Path::new(&path)).unwrap_or_else(|_| {});
        let path: &OsStr = OsStr::new(&path);
        let path = std::path::Path::new(&path);
        // std::fs::create_dir_all(&path.parent().unwrap()).unwrap();
        if let Some(path) = path.parent() {
            std::fs::create_dir_all(&path).unwrap();
        }
        let listener = UnixListener::bind(path).unwrap();

        if let Some(foo) = commands.get("term") {
            foo.run();
        }
        loop {
            if let Ok(job) = rx.try_recv() {
                match job {
                    Message::Quit => {
                        let mut sender = SENDER.lock().unwrap();
                        *sender = None;

                        // stop and delete socket
                        drop(listener);
                        std::fs::remove_file(std::path::Path::new(&path)).unwrap_or_else(|_| {});
                        break;
                    }
                    _ => {
                        eprintln!("not implemented: {:?}", job);
                    }
                }
            }
        }
    });

    return 0;
}

#[no_mangle]
pub extern "C" fn quit_rwm() -> c_int {
    let sender = SENDER.lock().unwrap();
    let ref_tx: &mpsc::Sender<Message> = match *sender {
        Some(ref x) => x,
        None => {
            eprintln!("thread not started, please first run init_rwm()");
            return 1;
        },
    };

    ref_tx.send(Message::Quit).unwrap();
    0
}


#[derive(Debug)]
pub enum Message {
    /// command to quit job
    Quit,
}