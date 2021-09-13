use std::fs;
use std::collections::{hash_map, HashMap};
use std::io::Write;

fn skip_to(start: u32, stop: u32) -> [(String, String); 4] {
    return [("factory".to_string(), "SkipAhead".to_string()), ("name".to_string(), "skip".to_string()), ("starttick".to_string(), start.to_string()), ("skiptotick".to_string(), stop.to_string())];
}

fn start_rec(start: u32) -> [(String, String); 4] {
    return [("factory".to_string(), "PlayCommands".to_string()), ("name".to_string(),"startrec".to_string()), ("starttick".to_string(), start.to_string()), ("commands".to_string(),"startrecording".to_string())];
}

fn stop_rec(stop: u32) -> [(String, String); 4] {
    return [("factory".to_string(), "PlayCommands".to_string()), ("name".to_string(),"stoprec".to_string()), ("starttick".to_string(), stop.to_string()), ("commands".to_string(),"stoprecording".to_string())];
}

fn next_demo(stop: u32, path: &str) -> [(String, String); 4] {
    return [("factory".to_string(), "PlayCommands".to_string()), ("name".to_string(),"nextdem".to_string()), ("starttick".to_string(), stop.to_string()), ("commands".to_string(),format!("playdemo {}", path))];
}

fn stop_demo(stop: u32) -> [(String, String); 4] {
    return [("factory".to_string(), "PlayCommands".to_string()), ("name".to_string(),"stopdem".to_string()), ("starttick".to_string(), stop.to_string()), ("commands".to_string(),"stopdemo".to_string())];
}

pub fn create_keyvalues(ticks: Vec<[u32; 2]>, next_path: &String) -> Vec<[(String, String); 4]> {
    let mut stack: Vec<[(String, String); 4]> = Vec::new();
    stack.push(skip_to(1, &ticks[0][0]-1));
    for (i, event) in ticks.iter().enumerate() {
        stack.push(start_rec(event[0]));
        stack.push(stop_rec(event[1]));
        if i+1 == ticks.len() {
            if next_path == "" {
                stack.push(stop_demo(ticks.last().unwrap().last().unwrap()+1)); //.last() is equivalent to [-1]/indexing last element. it may return None if a list is empty, so we .unwrap()
            } else {
                stack.push(next_demo(ticks.last().unwrap().last().unwrap()+1, &next_path));
            }
        } else {
            stack.push(skip_to(event[1]+1, ticks[i+1][0]-1));
        }
    }
    return stack;
}

pub fn write_vdf(path: &String, stack: Vec<[(String, String); 4]>) {
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.replace(".dem", ".vdm"))
        .unwrap();

    writeln!(f, "demoactions\n{{");
    for (i, action) in stack.iter().enumerate() {
        writeln!(f, "\t\"{}\"\n\t{{", i+1); //     "1"\n   {
        for (k, v) in action.iter() {
            writeln!(f, "\t\t{} \"{}\"", k, v); //I am so so sorry you have to read this
        }
        writeln!(f, "\t}}");
    }
    writeln!(f, "}}");

    //return f;
}