extern crate wifiscanner;
extern crate clap;
extern crate serde;
extern crate serde_json;

use std::error::Error;
use std::path::Path;
use std::fs::{File,OpenOptions};
use std::io::{stdin,stdout,Write,BufReader,BufWriter};
use std::time::{SystemTime,UNIX_EPOCH};
use std::convert::From;
use std::fmt;

use clap::{Arg,App,SubCommand};
use serde::{Deserialize, Serialize};

// TODO: Fix build errors from ScanMap.{read,write} refactor

/// Indicates position in coordinate system. It is suggested that x and y are positions in a horizontal 2D plane and z is the height.
#[derive(Serialize, Deserialize)]
struct Coordinate {
    x: f32,
    y: f32,
    z: f32,
}

impl Coordinate {
    /// Initializes a zero-ed Coordinate struct.
    fn new() -> Coordinate {
	   Coordinate{
	       x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

/// Network holds information about a wireless network.
#[derive(Serialize, Deserialize)]
struct Network {
    mac: String,
    ssid: String,
    channel: String,
    /// In dBm
    strength: String,
    /// Milliseconds since EPOCH
    time_scanned: u128,
}

/// Error which occurs during a wifi scan.
#[derive(Debug)]
struct ScanError {
    /// Reason scan failed.
    reason: wifiscanner::Error,
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	   write!(f, "resource={:?}", self.reason)
    }
}

impl Error for ScanError {}

impl From<wifiscanner::Error> for ScanError {
    /// Create a ScanError from a wifiscanner::Error.
    fn from(e: wifiscanner::Error) -> ScanError {
	   ScanError{
		  reason: e,
	   }
    }
}

impl Network {
    /// Scan wifi networks.
    fn scan() -> Result<Vec<Network>, Box<dyn Error>> {
	   let scan_time = (SystemTime::now().duration_since(UNIX_EPOCH)?).as_millis();
	   
        let scan = match wifiscanner::scan() {
		  Ok(s) => s,
		  Err(e) => return Err(Box::new(ScanError::from(e))),
	   };
	   
        let mut networks = Vec::<Network>::new();
        
        for network in scan {
            networks.push(Network{
                mac: network.mac,
                ssid: network.ssid,
                channel: network.channel,
                strength: network.signal_level,
                time_scanned: scan_time,
            });
        }

	   Ok(networks)
    }
}

/// Node is the result of a scan at a location.
#[derive(Serialize, Deserialize)]
struct Node {
    position: Coordinate,
    notes: String,
    networks: Vec<Network>,
}

impl Node {
    /// Create a new Node by asking the user for data and scanning.
    fn acquire() -> Result<Node, Box<dyn Error>> {
	   	   // Prompt user for position
        let mut position = Coordinate::new();
	   
	   let mut get_pos_done = false;
        while !get_pos_done {
            print!("x y z: ");
            stdout().flush().expect("failed to flush stdout");
            let mut pos_str = String::new();
            stdin().read_line(&mut pos_str).expect("failed to read input");
            pos_str = pos_str.replace("\n", "");

            let parts: Vec<&str> = pos_str.split(" ").collect();
            if parts.len() != 3 {
                println!("must be in format: x y z");
                continue;
            }

		  position.x = parts[0].parse::<f32>().expect("failed to parse x as float");
		  position.y = parts[1].parse::<f32>().expect("failed to parse y as float");
		  position.z = parts[2].parse::<f32>().expect("failed to parse z as float");
		  
            get_pos_done = true;
        }

        // Prompt user for notes
        print!("notes: ");

        let mut notes = String::new();
        stdout().flush().expect("failed to flush stdout");
        stdin().read_line(&mut notes).expect("failed to read input");
        notes = notes.replace("\n", "");

        // Scan networks
	   let networks = Network::scan()?;

	   if networks.len() == 0 {
		  println!("no networks found, maybe you need to run with sudo");
	   }

        Ok(Node{
            position: position,
            notes: notes,
            networks: networks,
        })
    }
}

/// Holds nodes with their scans. Saved to a file.
#[derive(Serialize, Deserialize)]
struct ScanMap {
    /// Title of the scan map.
    name: String,

    /// Free-form description of any additional details.
    notes: String,

    /// Scan data points.
    nodes: Vec<Node>,
}

impl ScanMap {
    /// Initialize an empty ScanMap.
    fn new() -> ScanMap {
	   ScanMap{
		  name: String::from(""),
		  notes: String::from(""),
		  nodes: Vec::<Node>::new(),
	   }
    }
    
    /// Creates a new ScanMap from an existing json file.
    fn read(p: &Path) -> Result<ScanMap, Box<dyn Error>> {
	   let file = File::open(p)?;
	   let reader = BufReader::new(file);

	   let scan_map = serde_json::from_reader(reader)?;

	   Ok(scan_map)
    }

    fn print_overview(&self) {
        println!("name: {}", self.name);
        println!("notes: {}", self.notes);
        println!("# nodes: {}", self.nodes.len());
    }


    /// Write curren ScanMap to .json file
    fn write(&self, p: &Path) -> Result<(), Box<dyn Error>> {
	   let file = OpenOptions::new().read(true).write(true).create(true).open(p)?;
	   let writer = BufWriter::new(file);

	   serde_json::to_writer(writer, self)?;

	   Ok(())
    }

    /// Acquire a new reading.
    fn acquire(&mut self) -> Result<(), Box<dyn Error>> {
	   let node = Node::acquire()?;
        
        println!("added node at ({}, {}, {}) with {} networks", node.position.x, node.position.y, node.position.z, node.networks.len());
        
        self.nodes.push(node);

	   Ok(())
    }
}

fn main() {
    // Command line arguments
    let arg_matches = App::new("Wifi Scanner")
        .about("Map wireless networks")
        .arg(Arg::with_name("map_file")
             .short("f")
             .long("map-file")
             .value_name("MAP_FILE")
             .help("File to save scan map")
             .takes_value(true)
             .required(true))
        .subcommand(SubCommand::with_name("record")
                    .about("Records a new scan to the map"))
        .get_matches();

    let map_file = arg_matches.value_of("map_file").unwrap();

    // Initialize scan map
    let map_file_path = Path::new(map_file);
    let mut scan_map = match map_file_path.exists() {
	   true => {
		  // Read existing scan map file
		  let scan_map = ScanMap::read(map_file_path).expect("failed to load existing scan map");

		  println!("loaded existing scan map \"{}\"", map_file_path.display());
		  scan_map.print_overview();
		  
		  scan_map
	   },
	   false => {
		  // Create new scan map
		  println!("creating new scan map \"{}\"", map_file_path.display());
		  
		  let mut scan_map = ScanMap::new();

		  let mut get_name_done = false;
		  while !get_name_done {
			 print!("name: ");
			 stdout().flush().expect("failed to flush stdout");
			 stdin().read_line(&mut scan_map.name)
				.expect("failed to read input");
			 scan_map.name = scan_map.name.replace("\n", "");

			 if scan_map.name.len() > 0 {
				get_name_done = true;
			 } else {
				println!("name cannot be empty");
			 }
		  }

		  print!("notes: ");
		  stdout().flush().expect("failed to flush stdout");
		  stdin().read_line(&mut scan_map.notes).expect("failed to read input");
		  scan_map.notes = scan_map.notes.replace("\n", "");

		  scan_map
	   },
    };

    if arg_matches.subcommand_matches("record").is_some() {
        // Acquire new reading
	   scan_map.acquire().expect("failed to acquire new reading");

        // Save scan map
        scan_map.write(map_file_path).expect("failed to save scan map");
    }
}
