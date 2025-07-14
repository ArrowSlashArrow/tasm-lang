use std::{collections::{HashMap, HashSet}, io::{stdout, Write}, time::{Duration, Instant}};
use serde::Deserialize;
use crossterm::{event::{self, Event, KeyCode}, terminal};
use std::{fs, env, process, fmt::{Formatter, Display, Result}};

// same as those defined in gdobj.py
const MEMREG: usize = 9998;
const PTRPOS: usize = 9999;

const CORNER: &str = "+";
const HORIZONTAL: &str = "-";
const VERTICAL: &str = "|";

const GRAY: &str = "\x1b[38;5;242m";
const RESET: &str = "\x1b[0m";
const YELLOW: &str = "\x1b[38;5;220m";
const BG_GREY: &str = "\x1b[48;5;238m";

const RED: &str = "\x1b[38;5;196m";
const GREEN: &str = "\x1b[38;5;46m";

const HIDE_CURSOR: &str = "\x1b[?25h";
const SHOW_CURSOR: &str = "\x1b[?25l";
const RESET_CURSOR_POS: &str = "\x1b[H";
// const CLEAR_SCREEN: &str = "\x1b[2J";
const CLEAR_LINE_AFTER_CURSOR: &str = "\x1b[0K";
const CLEAR_ALL_AFTER_CURSOR: &str = "\x1b[0J";

const CONTROLS_STRING: &str = "
------- Controls -------                 \x1b[0K
Space : play/pause                       \x1b[0K
    > : advance to next step while paused\x1b[0K
    ; : advance 10 steps while paused    \x1b[0K
    - : decrease speed                   \x1b[0K
    + : increase speed                   \x1b[0K
    q : exit                             \x1b[0K
"; // the clear line after cursor chars are here because you cannot format! a constant str


#[derive(Debug, Deserialize, Clone)]
struct Namespace {
    routines: HashMap<String, Routine>
}

#[derive(Debug, Deserialize, Clone)]
struct Routine {
    group: i32,
    instructions: Vec<Instruction>
}

#[derive(Debug, Deserialize, Clone)]
struct Instruction {
    command: String,
    idx: i32,
    args: Vec<String>
}

#[derive(Debug)]
struct Counter {
    id: i32,
    timer: bool
}

impl Counter {
    fn new<T: AsRef<str>>(s: T) -> Self {
        let inp = s.as_ref();
        let pref = inp.chars().next().unwrap();
        let id = match inp.char_indices().nth(1) {
            Some((i, _)) => {&inp[i..]},
            None => ""
        };
        Counter {id: id.parse::<i32>().unwrap(), timer: pref == 'T'}
    }
}

impl Display for Counter {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{}{}",
            match self.timer {
                true => "T",
                false => "C"
            },
            self.id
        )
    }
}

fn new_active(active_groups: &mut HashMap<i32, i32>, group: i32) {
    // pointer is set to -1 because it gets incremented to 0 immediately after
    active_groups.insert(group, -1);
}

// returns a routine object if a routine with a group exists, otherwise returns none
fn get_routine(namespace: &Namespace, group: i32) -> Option<Routine> {
    for (_, routine) in namespace.clone().routines.into_iter() {
        if routine.group == group {
            return Some(routine.clone())
        }
    }
    None
}

// simulate GD clamp
fn clamp(value: f64, isfloat: bool) -> f64 {
    if isfloat {
        if value > 9999999.0 {
            return 9999999.0
        } else {
            return value
        }
    } else {
        let newvalue = value.clamp(-2147483648.0, 2147483647.0);
        return newvalue
    }
}

// f64 is used here because it can hold both f32 and i32
fn get(counter: &Counter, counters: &[i32], timers: &[f32]) -> f64 {
    match counter.timer {
        true => timers[counter.id as usize] as f64,
        false => counters[counter.id as usize] as f64
    }
}

// counter = counter <op> value; op table: (0: =, 1: +, 2: -, 3: *, 4: /)
fn gsetv(counter: &Counter, rhsvalue: f64, op: i32, counters: &[i32], timers: &[f32]) -> f64 {
    let lhs = get(&counter, &counters, &timers);
    let result = match op {
        0 => rhsvalue,
        1 => lhs + rhsvalue,
        2 => lhs - rhsvalue,
        3 => lhs * rhsvalue,
        4 => {
            if rhsvalue != 0.0 {
                lhs / rhsvalue
            } else {
                0.0
            }
        },
        5 => {
            if rhsvalue != 0.0 {
                (lhs / rhsvalue).floor()
            } else {
                0.0
            }
        },
        _ => 0.0
    };

    clamp(result, counter.timer)
}

// counter = counter <op> rhs (0: =, 1: +, 2: -, 3: *, 4: /)
fn gsetc(counter: &Counter, rhs: &Counter, op: i32, counters: &[i32], timers: &[f32]) -> f64 {
    
    let lhs = get(&counter, &counters, &timers);
    let rhsvalue = get(&rhs, &counters, &timers);
    let value = match op {
        0 => rhsvalue,
        1 => lhs + rhsvalue,
        2 => lhs - rhsvalue,
        3 => lhs * rhsvalue,
        4 => {
            if rhsvalue != 0.0 {
                lhs / rhsvalue
            } else {
                0.0
            }
        },
        5 => {
            if rhsvalue != 0.0 {
                (lhs / rhsvalue).floor()
            } else {
                0.0
            }
        },
        _ => 0.0
    };

    clamp(value, counter.timer)
}

// counter = lhs <op> rhs (0: =, 1: +, 2: -, 3: *, 4: /)
fn gset2(result: &Counter, lhs_counter: Counter, rhs: Counter, op: i32, counters: &[i32], timers: &[f32]) -> f64 {
    let lhs = get(&lhs_counter, &counters, &timers);
    let rhsvalue = get(&rhs, &counters, &timers);
    let value = match op {
        0 => rhsvalue,
        1 => lhs + rhsvalue,
        2 => lhs - rhsvalue,
        3 => lhs * rhsvalue,
        4 => {
            if rhsvalue != 0.0 {
                lhs / rhsvalue
            } else {
                0.0
            }
        },
        5 => {
            if rhsvalue != 0.0 {
                (lhs / rhsvalue).floor()
            } else {
                0.0
            }
        },
        _ => 0.0
    };

    // clamp value
    clamp(value, result.timer)
}

// counter = lhs <op> mod (0: =, 1: +, 2: -, 3: *, 4: /)
fn gset2c(result: &Counter, lhs_counter: Counter, value: f64, op: i32, counters: &[i32], timers: &[f32]) -> f64 {
    let lhs = get(&lhs_counter, &counters, &timers);
    let newvalue = match op {
        0 => value,
        1 => lhs + value,
        2 => lhs - value,
        3 => lhs * value,
        4 => {
            if value != 0.0 {
                lhs / value
            } else {
                0.0
            }
        },
        5 => {
            if value != 0.0 {
                (lhs / value).floor()
            } else {
                0.0
            }
        },
        _ => 0.0
    };

    // clamp value
    clamp(newvalue, result.timer)
}

// display the state in a nice way
fn show_state(
    counters: &[i32], 
    timers: &[f32], 
    displayed_counters: &Vec<Counter>, 
    instructions: &Vec<Instruction>,
    memory_start: i32,
    memory_size: i32, 
    memory_mode: i32, 
    ptr_pos: i32,
    tick: u64,
    delay: f64,
    fast: bool,
    paused: bool,
    tick_time: Duration,
) {
    
    let (width, _) = terminal::size().expect("unable to get terminal size");
    let rows = 40;
    // clear screen
    let mut out_str = format!("{HIDE_CURSOR}{RESET_CURSOR_POS}");

    let memreg = counters[MEMREG];
    let ptrpos = counters[PTRPOS];

    // display memory if there is any
    if memory_size > 0 {
        let memcell_text_width = (memory_size - 1).to_string().len();
        let memcell_width = 16 + memcell_text_width;
        let first_memcell_width = memcell_width - 1;
        // amount of columns to display
        let columns = std::cmp::min((memory_size as f64 / rows as f64).ceil() as u16, width / memcell_width as u16);

        // top/bottom segments (first for first, next for all subsequent)
        let first_column = format!("{CORNER}{0:*^first_memcell_width$}{CORNER}{CLEAR_LINE_AFTER_CURSOR}", " MEMORY ").replace("*", HORIZONTAL);
        let next_column = HORIZONTAL.repeat(memcell_width - 1) + CORNER;
        
        // determine what memory addresses to show on what lines
        let mut lines: Vec<Vec<usize>> = vec![];
        for i in 0..memory_size {
            if i >= rows {
                lines[(i % rows) as usize].push(i as usize);
            } else {
                lines.push(vec![i as usize])
            }
        }

        // build the string for one specific memory address
        let build_memcell_str =|i: i32| {
            if i != ptr_pos {
                // addr: value
                format!(
                    " {i:0>width$}: {0}{GRAY}{1:0>14} {VERTICAL}",
                    if counters[(memory_start + i) as usize] < 0 {"-"} else {" "},
                    format!("{RESET}{}", counters[(memory_start + i) as usize].abs().to_string()),
                    width = memcell_text_width
                )
            } else {
                // highlight if pointer is here
                format!(
                    "{YELLOW} {BG_GREY}{0}> {1}{GRAY}{2:0>21}{RESET} {VERTICAL}",
                    " ".repeat(memcell_text_width),
                    if counters[(memory_start + i) as usize] < 0 {"-"} else {" "},
                    format!("{YELLOW}{}", counters[(memory_start + i) as usize].abs().to_string())
                )
            }
        };

        // top border of memory display
        out_str += &format!("{first_column}{}\n", next_column.repeat(columns as usize - 1usize));
        
        let mut i = 0;
        for line in lines.iter() {
            // add the memory cell strings for each line
            let mut column = "".to_string();
            for memcell in line.iter() {
                column += &build_memcell_str(*memcell as i32)
            }
            if i == memory_size % rows && memory_size % rows != 0 {
                let mut beginning = format!("{VERTICAL}{column}");
                beginning.pop();
                // add the bottom of the last row if it cuts off early
                out_str += &format!("{beginning}{CORNER}{next_column}{CLEAR_LINE_AFTER_CURSOR}\n")
            } else {
                out_str += &format!("{VERTICAL}{column}{CLEAR_LINE_AFTER_CURSOR}\n")
            }
            
            i += 1;
        }
        
        // add the bottom of the memory cell display
        let mut bottom_row = format!("{CORNER}{}", next_column.repeat((memory_size / rows) as usize));
        
        // the corner of the register / pointer / writemode display
        if bottom_row.len() < 25 {
            bottom_row += &format!("{}{CORNER}", HORIZONTAL.repeat(24 - bottom_row.len()))
        } else {
            unsafe {
                bottom_row.as_mut_vec()[24] = b'+';
            }
        }

        // add the bottom right corner of the main memory cell display
        if columns == 1 {
            unsafe {
                bottom_row.as_mut_vec()[18] = b'+';
            }
        }

        let mode_str = match memory_mode {
            1 => format!("{GREEN} READ"),
            2 => format!("{RED}WRITE"),
            _ => "?????".to_string()
        };
        
        // build the register / pointer / writemode display
        out_str += &format!("{bottom_row}{CLEAR_LINE_AFTER_CURSOR}\n");
        out_str += &format!(
            "{VERTICAL} Register: {0}{GRAY}{1:0>14} {VERTICAL}{CLEAR_LINE_AFTER_CURSOR}\n", 
            match memreg < 0 {true => "-", false => " "}, 
            format!("{RESET}{}", memreg.abs())
        );
        out_str += &format!("{VERTICAL} Pointer:   {ptrpos:>10} {VERTICAL}{CLEAR_LINE_AFTER_CURSOR}\n");
        out_str += &format!("{VERTICAL} Pointer mode:   {mode_str} {RESET}{VERTICAL}{CLEAR_LINE_AFTER_CURSOR}\n");
        out_str += &format!("+-----------------------+{CLEAR_LINE_AFTER_CURSOR}\n{CLEAR_LINE_AFTER_CURSOR}\n");
    }
    
    if displayed_counters.len() > 0 {
        let left_len = 5;
        let mut right_len_int = 0;
        let mut float_lengths: Vec<usize> = vec![];
        
        // determine how wide the dispaly should be
        for counter in displayed_counters.iter() {
            if counter.timer {
                float_lengths.push(std::cmp::min((timers[counter.id as usize] % 1.0).to_string().len(), 2usize))
            } else {
                let length = counters[counter.id as usize].to_string().len();
                if right_len_int < length {
                    right_len_int = length
                }
            }
        }
        let right_len_float = if float_lengths.len() > 0 {*float_lengths.iter().max().unwrap() as i32} else{-1};
        let length = (6 + left_len as i32 + right_len_int as i32 + right_len_float) as usize;

        // top border
        out_str += &format!("{CORNER}{0:*^length$}{CORNER}{CLEAR_LINE_AFTER_CURSOR}\n", " COUNTERS ").replace('*', HORIZONTAL);

        let mut right_padding = false;
        for counter in displayed_counters.iter() {
            if counter.timer {
                right_padding = true
            }
        }
        
        // then display
        for counter in displayed_counters.iter() {
            let value = get(counter, &counters, &timers);
            let counter_str = format!("{}{}", if counter.timer {"T"} else {"C"}, counter.id);
            out_str += &format!("{VERTICAL} {counter_str:<left_len$} {VERTICAL} {0:>right_len_int$}", (value as i32).to_string());
            if right_len_float > -1 && counter.timer {
                out_str += &format!(".{0:0>2} {VERTICAL}{CLEAR_LINE_AFTER_CURSOR}\n", (value * 100.0) as i32 % 100)
            } else {
                out_str += &format!("{} {VERTICAL}{CLEAR_LINE_AFTER_CURSOR}\n", if right_padding {"   "} else {""});
            }
        }

        // bottom border
        out_str += &format!("{CORNER}{0}{CORNER}{CLEAR_LINE_AFTER_CURSOR}\n", "-".repeat(length))
    }

    if instructions.len() > 0 {
        let caption = " Instructions this tick ";

        let mut instruction_lines = vec![];
        let mut display_width = 0;
        for instruction in instructions {
            // format instruction as it is in the file
            let mut line = format!("{} ", instruction.command);
            for arg in &instruction.args {
                line += &format!("{arg}, ");
            }
            // remove last comma
            line.pop();
            line.pop();

            if line.len() > display_width {
                display_width = line.len()
            }
            instruction_lines.push(line);
        }

        display_width = std::cmp::max(display_width, caption.len());

        // top border
        out_str += &format!("{CLEAR_LINE_AFTER_CURSOR}\n{CORNER}{HORIZONTAL}{caption:*<display_width$}{HORIZONTAL}{CORNER}{CLEAR_LINE_AFTER_CURSOR}\n").replace("*", HORIZONTAL);

        for line in instruction_lines {
            out_str += &format!("{VERTICAL} {line: <display_width$} {VERTICAL}{CLEAR_LINE_AFTER_CURSOR}\n");
        }

        // bottom border
        out_str += &format!("{CORNER}{}{CORNER}{CLEAR_LINE_AFTER_CURSOR}\n", HORIZONTAL.repeat(display_width + 2usize));
        
        out_str += &format!("{CLEAR_LINE_AFTER_CURSOR}\n");
    }

    // time and speed dispaly
    if !fast {
        out_str += &format!(
            "Time: {:.3}s / {tick} ticks{CLEAR_LINE_AFTER_CURSOR}\nSimulation speed: {:.2}Hz ({:.2}x) {}{CLEAR_LINE_AFTER_CURSOR}", 
            tick as f64 / 240.0,             // time in seconds
            1000.0 / delay,                  // simulation steps per second
            1000.0 / delay / 240.0,          // how much faster it is than GD
            if paused {"[PAUSED]"} else {""} // paused?
        );
    } else {
        let delay = tick_time.as_nanos() as f64 / 1000000.0;
        out_str += &format!(
            "Time: {:.3}s / {tick} ticks{CLEAR_LINE_AFTER_CURSOR}\nRunning simulation as fast as possible: {:.2}Hz ({:.2}x) @ {delay:.4}ms / tick {}{CLEAR_LINE_AFTER_CURSOR}", 
            tick as f64 / 240.0,             // time in seconds
            1000.0 / delay,                  // simulation steps per second
            1000.0 / delay / 240.0,          // how much faster it is than GD
            if paused {"[PAUSED]"} else {""} // paused?
        );
    }
    out_str += CONTROLS_STRING;
    // clear all after to prevent weird overdraw
    out_str += &format!("{CLEAR_ALL_AFTER_CURSOR}{SHOW_CURSOR}\n"); 

    stdout().write_all(out_str.as_bytes()).unwrap();
    stdout().flush().unwrap();
}

fn main() {
    // let start = Instant::now();

    let mut paused = false;
    let default_delay = 4.166667f64; // default speed of geometry dash 
    let mut delay = default_delay; // how much time to wait between ticks in ms
    let mut _extra_steps = 0;

    let argv = env::args().collect::<Vec<String>>();
    let infile = argv.get(1).expect("No input filepath found (argument 1).");
    let file = fs::read_to_string(&infile).expect("Unable to read file.");    

    let fast = argv.contains(&"--fast".to_string());

    // read raw namespace to object
    let raw_namespace: Namespace = serde_json::from_value(
        serde_json::from_str(&file).expect("Could not parse json.")
    ).unwrap();

    // simulation state
    let mut counters: [i32; 10000] = [0; 10000];  // there is 1 extra 0 so that indices correspond to item ids directly
    let mut timers: [f32; 10000] = [0.0; 10000];  // there is 1 extra 0 so that indices correspond to item ids directly
    let mut displayed_counters: Vec<Counter> = vec![];
    let mut memory_start: i32 = 0; // starting counter of memory
    let mut memory_size: i32 = 0;
    let mut memory_mode: i32 = 0;  // MREAD / MWRITE
    let mut ptr_pos: i32 = 0;

    let mut active_groups: HashMap<i32, i32> = HashMap::new();

    // check start routine
    if !raw_namespace.routines.contains_key("_start") {
        // you should be including the _start routine.
        println!("No _start routine found. this program refuses to interpret such code.");
        return;
    }
    active_groups.insert(raw_namespace.routines.get("_start").unwrap().group, 0);

    // this value equals 2^(1/5) -> 5x increase speed button pressed = 2x speed overall
    let speed_multiplier = 1.148698355;

    // init
    let init_routine = raw_namespace.routines.get("_init").unwrap();
    let mut malloced = false;
    let mut idx = 0;

    // process init routine
    for instruction in init_routine.instructions.clone().into_iter() {
        let command = instruction.command.as_str();
        let args: Vec<String> = instruction.args;
        match command {
            "MALLOC" => {
                if malloced {
                    println!("[Instruction {idx} in _init] You cannot allocate memory twice.");
                    return
                }
                let length = args[0].parse::<i32>().unwrap();
                memory_start = MEMREG as i32 - length;
                memory_size = length;
                malloced = true;
            },
            "INITMEM" => {
                if !malloced {
                    println!("[Instruction {idx} in _init] You cannot initialise unallocated memory. Please MALLOC first.");
                    return
                }
                let new_state = args[0].split(",");
                let mut index = 0;
                for number in new_state {
                    counters[(memory_start + index) as usize] = number.parse::<i32>().unwrap();
                    index += 1;
                    if index > memory_size {
                        println!("[Instruction {idx} in _init] You cannot initialise more slots of memory than you allocated.");
                        return
                    }
                }
            },
            "DISPLAY" => {
                displayed_counters.push(
                    Counter::new(args[0].as_str())
                )
            },
            _ => {}
        }
        idx += 1;
    }

    let mut routines: HashMap<String, Routine> = HashMap::new();

    // replace all the MEMSIZE with memory_size
    for (name, routine) in raw_namespace.routines.iter() {
        let old_instrs = &routine.instructions;
        let group = &routine.group;
        let mut instructions: Vec<Instruction> = vec![];

        for instruction in old_instrs.iter() {
            let mut new_args: Vec<String> = vec![];
            for arg in instruction.args.clone() {
                if arg == "MEMSIZE" {
                    new_args.push(memory_size.to_string());
                } else {
                    new_args.push(arg.clone());
                }
            }
            instructions.push(
                Instruction { 
                    command: instruction.command.clone(), 
                    idx: instruction.idx, 
                    args: new_args 
                }
            )
        }
        routines.insert(
            name.clone(),
            Routine {
                group: *group,
                instructions: instructions
            }
        );
    }

    let namespace = Namespace { 
        routines: routines
    };


    let mut tick: u64 = 0;
    let mut previous_inputs: HashSet<KeyCode> = HashSet::new();
    let mut prev_tick_time = Duration::new(0, 0);
    let mut tick_time = Duration::new(0, 0);
    let mut current_instructions: Vec<Instruction> = vec![];
    while !active_groups.is_empty() {
        let start_tick_time = Instant::now();
        show_state(
            &counters, 
            &timers, 
            &displayed_counters, 
            &current_instructions,
            memory_start, 
            memory_size, 
            memory_mode, 
            ptr_pos, 
            tick,
            delay,
            fast,
            paused,
            prev_tick_time
        );
        _extra_steps = 0;

        // get input 
        if event::poll(Duration::from_millis(0)).unwrap() {
            if let Event::Key(keystroke) = event::read().unwrap() {
                if !previous_inputs.contains(&keystroke.code) {
                    match keystroke.code {
                        KeyCode::Char('q') => {
                            process::exit(0);
                        },
                        KeyCode::Char(' ') => {
                            paused = !paused;
                        },
                        KeyCode::Char('-') => {
                            delay *= 1.148698355;
                        },
                        KeyCode::Char('=') => delay /= speed_multiplier,
                        KeyCode::Char('.') => _extra_steps += 1,
                        KeyCode::Char(';') => _extra_steps += 10,
                        _ => {}
                    };
                    // min value is default delay / 64
                    // max value is default delay * 64
                    delay = delay.clamp(0.0651041, 266.666667);
                    previous_inputs.insert(keystroke.code);
                } else {
                    previous_inputs.remove(&keystroke.code);
                }
            } 
        }
        // determine
        let steps = match paused {
            true => _extra_steps,
            false => 1
        };
        
        for _ in 0..steps {
            // let tick_start = Instant::now();
            tick += 1;
            current_instructions.clear();
            
            for (group, ptr) in active_groups.iter() {
                // find instruction (group.instructions[ptr])
                let routine = get_routine(&namespace, *group).unwrap();
                current_instructions.push(routine.instructions[*ptr as usize].clone());
            }
            
            // process all instructions
            for instruction in current_instructions.clone().into_iter() {
                let command = instruction.command.as_str();
                let mode = instruction.idx;
                let args: Vec<String> = instruction.args;

                // i had to fight the borrow checker for these closures
                // but it was worth it to remove duplicate code

                let mut arithmetic = |op| {
                    let result = Counter::new(&args[0]);
                    let resulting_value = match mode {
                        1 => { // item = num
                            gsetv(&result, args[1].parse::<f64>().unwrap(), op, &mut counters, &mut timers)
                        },
                        2 => { // item = item
                            let rhs = Counter::new(&args[1]);
                            gsetc(&result, &rhs, op, &mut counters, &mut timers)
                        },
                        3 => {
                            let lhs = Counter::new(&args[1]);
                            let rhs = Counter::new(&args[2]);
                            gset2(&result, lhs, rhs, op, &mut counters, &mut timers)
                        },
                        4 => {
                            let lhs = Counter::new(&args[1]);
                            let rhs = args[2].parse::<f64>().unwrap();
                            gset2c(&result, lhs, rhs, op, &mut counters, &mut timers)
                        }
                        _ => 0.0
                    };
                    match result.timer {
                        true => {
                            timers[result.id as usize] = resulting_value as f32;
                        },
                        false => {
                            counters[result.id as usize] = resulting_value as i32;
                        }
                    };
                };

                let mut spawns = |compare: i32, counters: &mut [i32], timers: &mut [f32]| {
                    let value = match mode {
                        1 => args[2].parse::<f64>().unwrap(),
                        2 => get(&Counter::new(&args[2]), &counters, &timers),
                        _ => 0.0
                    };
                    let lhs = get(&Counter::new(&args[1]), &counters, &timers);
                    if match compare {
                        0 => lhs == value,
                        1 => lhs > value,
                        2 => lhs >= value,
                        3 => lhs < value,
                        4 => lhs <= value,
                        5 => lhs != value,
                        _ => false
                    } {
                        new_active(&mut active_groups, namespace.routines.get(&args[0]).unwrap().group);
                    }
                };

                let forks = |compare: i32, counters: &mut [i32], timers: &mut [f32], active_groups: &mut HashMap<i32, i32>| {
                    let value = match mode {
                        1 => args[3].parse::<f64>().unwrap(),
                        2 => get(&Counter::new(&args[3]), &counters, &timers),
                        _ => 0.0
                    };
                    let lhs = get(&Counter::new(&args[2]), &counters, &timers);
                    if match compare {
                        0 => lhs == value,
                        1 => lhs > value,
                        2 => lhs >= value,
                        3 => lhs < value,
                        4 => lhs <= value,
                        5 => lhs != value,
                        _ => false
                    } {
                        new_active(active_groups, namespace.routines.get(&args[0]).unwrap().group);
                    } else {
                        new_active(active_groups, namespace.routines.get(&args[1]).unwrap().group);
                    }
                };

                match command {
                    "SPAWN"  => new_active(&mut active_groups, namespace.routines.get(&args[0]).unwrap().group),
                    "MREAD"  => memory_mode = 1,
                    "MWRITE" => memory_mode = 2,
                    "MPTR"   => ptr_pos += args[0].parse::<i32>().unwrap(),
                    "MRESET" => ptr_pos = 0,
                    "MFUNC"  => {
                        match memory_mode {
                            1 => { // read
                                counters[MEMREG] = counters[(memory_start + ptr_pos) as usize];
                            },
                            2 => { // write
                                counters[(memory_start + ptr_pos) as usize] = counters[MEMREG];
                            },
                            _ => {}
                        }
                    }
                    "NOP"    => {},
                    "MOV"    => arithmetic(0),
                    "ADD"    => arithmetic(1),
                    "SUB"    => arithmetic(2),
                    "MUL"    => arithmetic(3),
                    "DIV"    => arithmetic(4),
                    "FLDIV"  => arithmetic(5),
                    "SE"     => spawns(0, &mut counters, &mut timers),
                    "SG"     => spawns(1, &mut counters, &mut timers),
                    "SGE"    => spawns(2, &mut counters, &mut timers),
                    "SL"     => spawns(3, &mut counters, &mut timers),
                    "SLE"    => spawns(4, &mut counters, &mut timers),
                    "SNE"    => spawns(5, &mut counters, &mut timers),
                    "FE"     => forks(0, &mut counters, &mut timers, &mut active_groups),
                    "FG"     => forks(1, &mut counters, &mut timers, &mut active_groups),
                    "FGE"    => forks(2, &mut counters, &mut timers, &mut active_groups),
                    "FL"     => forks(3, &mut counters, &mut timers, &mut active_groups),
                    "FLE"    => forks(4, &mut counters, &mut timers, &mut active_groups),
                    "FNE"    => forks(5, &mut counters, &mut timers, &mut active_groups),
                    _ => {}
                }

                // stuff like limits, special counters and whatnot
                if ptr_pos < 0 {
                    ptr_pos = 0
                } else if ptr_pos >= memory_size {
                    ptr_pos = memory_size - 1
                }
                counters[PTRPOS] = ptr_pos;
            }

            // move all group pointers forward
            let mut remove_groups: Vec<i32> = vec![];
            for (group, ptr) in active_groups.iter_mut() {
                *ptr += 1;
                let routine = get_routine(&namespace, *group).unwrap();
                if routine.instructions.len() <= *ptr as usize {
                    remove_groups.push(*group);
                }
            }
            
            // group is not active if it has reached the end of its instruction set
            for group in remove_groups.iter() {
                active_groups.remove(&group);
            }
            
            prev_tick_time = tick_time;
            tick_time = start_tick_time.elapsed();

            // wait logic
            let now = Instant::now();
            let wait_until = start_tick_time + Duration::from_nanos((delay * 1000000.0) as u64);
            
            if now < wait_until && !fast {
                // wait
                while Instant::now() < wait_until {}
            } else {
                // lag happened
            }
        }    
    }
}
