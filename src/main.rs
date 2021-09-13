mod vdf;

use bitbuffer::BitRead;
use main_error::MainError;
use std::{fs, io};
use std::env;
use tf_demo_parser::demo::header::Header;
use tf_demo_parser::demo::parser::{DemoHandler, RawPacketStream, DemoParser};
use tf_demo_parser::{Demo, MatchState};
use text_io;
use std::io::Bytes;
use crate::vdf::create_keyvalues;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fmt;
//use alloc::vec::IntoIter;
//use std::alloc::Global;
//use std::io::Write;

//I don't know what this is for but the Demos.tf parser binary uses it so it must be important
#[cfg(feature = "jemallocator")]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn find_start(demo_state: &MatchState) -> u32 {
    /*
    &state is implemented in parser/parser/analyser.rs, it is a MatchState. I think it is called this because it is something you can
    use in the match statement? Anyway, it contains properties chat,users,deaths,rounds,start_tick,interval_per_tick.
    Each of these properties is a struct. into_iter makes these structs into an iter. find is a way to find a value based on
    a predicate. I'm not sure, but I think (|x| x.a == 5) is the equivalent of Python's lambda x: x.a == 5.
    This is insanely weird syntax. Am I the only one that thinks this.
    This returns what is called an Option, because it may be a ChatMessage or it may be null.
    */
    let findmsg = demo_state.chat.iter().find(|x| x.text == "[SOAP] Soap DM unloaded.");
    //Some(x) => ... means do something for the value x. This can be split into variable paths, like
    //Some(x > 0) => ...
    //Some(x < 0) => ...
    //Some(x == 0) => ...
    return match findmsg {
        Some(msg) => msg.tick, //If you find something matching the predicate, return its tick property.
        None => demo_state.start_tick //If the demo does not include a SOAP portion, assume it starts at the tick given in the demo as a start tick. This is not 1, and I'm not sure the significance of it, but it seems like a safe bet to use it.
    };
}

fn find_my_streaks(demo_state: &MatchState, my_steam3: &str, cfg: &Config, starttick: u32) -> Vec<[u32; 2]> {
    let my_id = match &demo_state.users.iter().find(|(_, y)| y.steam_id == my_steam3) {
        Some((_,y)) => Ok(y.user_id),
        None => Err("User with matching Steam3 not found")
    };

    let my_kills = demo_state.deaths.iter().filter(|x| x.killer == my_id.unwrap() && x.tick > starttick);

    let mut runs = Vec::new();
    let mut start = 0; //What tick did this streak start at
    let mut last = 0; //What was the last tick we examined
    let mut iter = 1; //How many kills this run
    for i in my_kills {
        if start == 0 {start = i.tick}; //first kill could be part of streak
        let tick = i.tick;
        let diff = tick - last;
        if diff < cfg.space_btwn { //If this kill is part of a streak
            iter = iter + 1; //increase!
        } else { //Too long between kills - end streak and record it, if desired
            if iter >= cfg.min_kills { //If enough kills to constitute a streak
                runs.push([start - cfg.start_before, last + cfg.hang_after]);
            }
            //either way, we're going to set this kill as the start of a new streak.
            start = tick;
            iter = 1;
        }

        last = tick;
    }
    return runs;
}

struct Config {
    space_btwn: u32,
    start_before: u32,
    hang_after: u32,
    min_kills: u32
}

fn process_file(path: String, steam3: &String, cfg: &Config, next_path: &String) -> Result<(), MainError> {
    let file = fs::read(&path)?;

    let demo = Demo::new(&file);
    let mut handler = DemoHandler::default();

    let mut stream = demo.get_stream();
    let header = Header::read(&mut stream)?;
    handler.handle_header(&header);

    let mut packets = RawPacketStream::new(stream);

    while let Some(packet) = packets.next(&handler.state_handler)? {
        handler.handle_packet(packet).unwrap();
    }

    assert_eq!(false, packets.incomplete);
    let parser = DemoParser::new(demo.get_stream());
    let (_, state) = parser.parse()?;

    let starttick: u32 = find_start(&state); //we use references otherwise they're a "moved resource" and we cant reuse it

    let streaks = find_my_streaks(
        &state,
        &*steam3,
        &cfg,
        starttick);
    if streaks.len() == 0 {
        println!("No streaks ({})", path);
        return Ok(())
    } else {
        println!("Found {} streaks ({})", streaks.len(), path);
        let kvs: Vec<[(String, String); 4]> = create_keyvalues(streaks, next_path);
        vdf::write_vdf(&path, kvs);
    }
    return Ok(())
}

fn main() -> Result<(), MainError> {
    //let args: Vec<String> = env::args().collect();//you could use this but getting input during run seems more user friendly
    println!("Enter your path your demo files");
    let path: String = text_io::read!();
    println!("Enter your Steam3 ID");
    let steam3: String = text_io::read!();

    println!("Minimum amount of kills to form a killstreak: ");
    let min_kills: u32 = text_io::read!();
    println!("Max ticks between kills to form a killstreak (i.e. 300, 500, 1000): ");
    let ticks_between: u32 = text_io::read!();
    println!("How many ticks before the first kill to begin recording? (i.e. 500): ");
    let ticks_before: u32 = text_io::read!();
    println!("How many ticks after the last death to end recording? (i.e. 150): ");
    let ticks_after: u32 = text_io::read!();

    let cfg = Config{space_btwn: ticks_between, start_before: ticks_before, hang_after: ticks_after, min_kills: min_kills};

    let mut entries = fs::read_dir(path)?
        .map(|res| res.map(|e| e.path()))//I believe this converts each entry inside a Result object to its path inside a Result object
        .collect::<Result<Vec<_>, io::Error>>()?;
    entries.sort(); //this and the mapping are necessary because simply doing .read_dir() does not guarantee order.
    let entriesFiltered = entries.iter().filter(|x| match x.extension() {
        Some(ext) => ext == "dem", //if there is an extension, return value of the boolean (value goes to the filter)
        _ => false, //if no extension, just return false to the filter (so its not included)
    }).collect::<Vec<&PathBuf>>();

    println!("Beginning processing of {} files", entriesFiltered.len());
    let mut last: String = "".to_string();
    for (idx, window) in entriesFiltered.windows(2).enumerate() { //returns items in the collection in pairs of 2
        let e: String = window[0].to_str().unwrap().parse().unwrap();
        let n: String = window[1].to_str().unwrap().parse().unwrap();
        process_file(e, &steam3, &cfg, &n);
        last = n; //this was easier than just indexing the last element because Rust sucks
    }
    process_file(last, &steam3, &cfg, &"".to_string());
    println!("Done processing");
    return Ok(())
}
