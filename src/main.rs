extern crate wifiscanner;
extern crate clap;
extern crate serde;
extern crate serde_json;

use std::path::Path;
use std::fs;
use std::io;
use std::io::Write;
use std::ffi::OsStr;

use clap::{Arg,App};
use serde::{Deserialize, Serialize};

// Indicates position in coordinate system.
// It is suggested that x and y are positions in a horizontal 2D plane and
// z is the height.
#[derive(Serialize, Deserialize)]
struct Coordinate {
    x: f32,
    y: f32,
    z: f32,
}

// Network holds information about a wireless network.
#[derive(Serialize, Deserialize)]
struct Network {
    mac: String,
    ssid: String,
    channel: String,
    // In dBm
    strength: String,
}

// Node is the result of a scan at a location.
#[derive(Serialize, Deserialize)]
struct Node {
    position: Coordinate,
    networks: Vec<Network>,
    notes: String,
}

// Holds nodes with their scans. Saved to a file.
#[derive(Serialize, Deserialize)]
struct ScanMap {
    name: String,
    notes: String,    
    nodes: Vec<Node>,
}

impl ScanMap {
    fn print_overview(&self) {
        println!("name: {}", self.name);
        println!("notes: {}", self.notes);
        println!("# nodes: {}", self.nodes.len());
    }
}

fn main() {
    // Command line arguments
    let arg_matches = App::new("Wifi Scanner")
        .version("0.1.0")
        .author("Noah Huppert")
        .about("Map wireless networks")
        .arg(Arg::with_name("map_file")
             .short("f")
             .long("map-file")
             .value_name("MAP_FILE")
             .help("File to save scan map")
             .takes_value(true)
             .required(true))
        .get_matches();

    let map_file = arg_matches.value_of("map_file").unwrap();

    // Load map file if it exists
    let map_file_path = Path::new(map_file);
    if map_file_path.extension().unwrap_or(OsStr::new("")) != OsStr::new("json") {
        panic!("map file must have a .json extension")
    }
    
    let mut scan_map = ScanMap{
        name: "".to_string(),
        notes: "".to_string(),
        nodes: Vec::<Node>::new(),
    };
    
    if map_file_path.exists() {
        let scan_map_str = match fs::read_to_string(map_file_path) {
            Ok(v) => v,
            Err(e) => panic!("failed to read existing scan map \"{}\": {}",
                             map_file, e),
        };
        
        scan_map = match serde_json::from_str(&scan_map_str.to_owned()) {
            Ok(v) => v,
            Err(e) => panic!("failed to JSON parse existing scan map \"{}\": {}",
                             map_file, e),
        };

        println!("loaded existing scan map \"{}\"", map_file);
        scan_map.print_overview();
    } else {
        println!("creating new scan map \"{}\"", map_file);
        
        print!("name: ");
        io::stdout().flush().expect("failed to flush stdout");
        io::stdin().read_line(&mut scan_map.name).expect("failed to read input");
        scan_map.name = scan_map.name.replace("\n", "");

        print!("notes: ");
        io::stdout().flush().expect("failed to flush stdout");
        io::stdin().read_line(&mut scan_map.notes).expect("failed to read input");
        scan_map.notes = scan_map.notes.replace("\n", "");
    }
    
    let scan = wifiscanner::scan();

    // Save scan map
    let scan_map_str = match serde_json::to_string(&scan_map) {
        Ok(v) => v,
        Err(e) => panic!("failed to JSON serialize scan map: {}", e),
    };

    match fs::write(map_file_path, scan_map_str) {
        Ok(()) => (),
        Err(e) => panic!("failed to save scan map to \"{}\": {}", map_file, e),
    };
}
