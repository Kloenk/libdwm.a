use std::ffi::*;
use std::os::raw::*;

use std::io::{BufRead, BufReader};
use std::os::unix::prelude::*;
use std::io::prelude::*;
use std::os::unix::net::{UnixStream, UnixListener};

use std::collections::HashMap;
use std::thread;
use std::process;

use std::sync::mpsc;
use std::sync::Mutex;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref SENDER: Mutex<Option<mpsc::Sender<Message>>> = Mutex::new(None);
    static ref RECEIVER: Mutex<Option<mpsc::Receiver<Message>>> = Mutex::new(None);
    static ref HANDLE: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);
}



/// Command struct holding a command to execute
#[repr(C)]
pub struct Command_r {
    name: *const c_char,
    func: fn(arg: *const Arg_r),
    arg: Arg_r,
}

#[derive(Clone, Debug)]
struct Command {
    name: String,
    func: fn(arg: *const Arg_r),
    arg: Arg,
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
            let arg = Arg::fromArg_r(v.arg);

            ret.insert(name.clone(), Self{
                name,
                func,
                arg,
            });
        }

        ret
    }

    unsafe fn run(&self) {
        let func: fn(arg: *const Arg_r) = self.func;
        func(self.arg.to_arg());
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

#[derive(Copy, Clone, Debug)]
enum Arg {
    i(i32),
    ui(u32),
    f(f32),
    v(usize),
}

impl Arg {
    unsafe fn fromArg_r(arg: Arg_r) -> Self {
        match arg {
            Arg_r { v } => return Arg::v(v as *const usize as usize),
            Arg_r { ui } => return Arg::ui(ui),
            Arg_r { f } => return Arg::f(f),
            Arg_r { i } => return Arg::i(i),
        }
    }
    unsafe fn to_arg(&self) -> *const Arg_r {
        match self {
            Arg::f(f) => return &Arg_r { f: *f},
            Arg::i(i) => return &Arg_r { i: *i},
            Arg::ui(ui) => return &Arg_r { ui: *ui},
            _ => return &Arg_r { f: 0.0},
        }
    }
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
pub extern "C" fn run_rwm() -> c_int {
    let mut receiver = RECEIVER.lock().unwrap();
    if let Some(rx) = receiver.take() {
        if let Ok(job) = rx.try_recv() {
            match job {
                Message::Command(cmd) => {
                    println!("run command {}", cmd.name);
                    unsafe { cmd.run() };
                }
                _ => (),
            }
        }
        *receiver = Some(rx);
        return 0;   
    }
    1
}

#[no_mangle]
pub extern "C" fn init_rwm(path: *const c_char, commands: *const Command_r, len: c_int) -> c_int {
    let (tx, rx): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel();

    let mut sender = SENDER.lock().unwrap();
    *sender = Some(tx);
    drop(sender);

    let (tx2, rx2): (mpsc::Sender<Message>, mpsc::Receiver<Message>) = mpsc::channel();

    let mut receiver = RECEIVER.lock().unwrap();
    *receiver = Some(rx2);
    drop(receiver);

    let path = unsafe { CStr::from_ptr(path).to_string_lossy() };
    let path = parse_vars(&path);

    // parse commands
    /* let commands = unsafe { std::slice::from_raw_parts(commands, len as usize)};
    let name = commands[1].name;
    let name = unsafe { CStr::from_ptr(name).to_string_lossy()};
    let function: fn(arg: *const Arg_r) = commands[1].func as fn(arg: *const Arg_r);
    function(&commands[1].arg); */
    let commands = unsafe { Command::from_ptr(commands, len)};

    let mut handle = HANDLE.lock().unwrap();
    let join = std::thread::spawn(move || {
        std::fs::remove_file(std::path::Path::new(&path)).unwrap_or_else(|_| {});
        let path: &OsStr = OsStr::new(&path);
        let path = std::path::Path::new(&path);
        // std::fs::create_dir_all(&path.parent().unwrap()).unwrap();
        if let Some(path) = path.parent() {
            std::fs::create_dir_all(&path).unwrap();
        }
        let listener = UnixListener::bind(path).unwrap();
        listener.set_nonblocking(true).unwrap();

        if let Some(foo) = commands.get("toggletag") {
            println!("run quit");
            tx2.send(Message::Command(foo.clone()));
        }
        loop {
            let recv = rx.try_recv();
            if let Ok(job) = recv {
                let job: Message = job;
                match job {
                    Message::Quit => {
                        SENDER.lock().unwrap().take();

                        println!("quit");

                        // stop and delete socket
                        std::fs::remove_file(std::path::Path::new(&path)).unwrap_or_else(|_| {});
                        break;
                    }
                    _ => {
                        eprintln!("not implemented: {:?}", job);
                    }
                }
            }
            if let Ok((conn, _)) = listener.accept() {
                std::thread::sleep(std::time::Duration::from_millis(40));
                let mut stream: UnixStream = conn;
                let mut input = String::new();
                stream.read_to_string(&mut input).unwrap();
                let input = input.trim();
                println!("got command: {} {:?}", input, commands);
                if let Some(cmd) = commands.get(input) {
                    println!("execute command {}", cmd.name);
                    //stream.write(b"ok").unwrap();
                    tx2.send(Message::Command(cmd.clone()));
                } else {
                    println!("unknown command");
                    /* stream.write(b"unknown command").unwrap_or_else(|err| {
                        println!("error writing status: {}", err);
                        0
                    }); */
                }
            } 
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
    });
    *handle = Some(join);


    println!("giveing thread back");
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

    println!("send quit command");

    ref_tx.send(Message::Quit).unwrap();
    drop(ref_tx); drop(sender);

    let mut handle = HANDLE.lock().unwrap();
    handle.take().unwrap().join().unwrap();
    0
}


#[derive(Debug)]
enum Message {
    /// command to quit job
    Quit,

    /// Command to execute
    Command(Command),
}