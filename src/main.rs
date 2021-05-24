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
use std::process::exit;

use clap::{Arg,App,SubCommand,ArgMatches};
use serde::{Deserialize, Serialize};

/// Print an error message to stderr and exit the process with exit code 1.
fn die(msg: &str) {
    eprintln!("Error: {}", msg);
    exit(1);
}

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
#[derive(Serialize,Deserialize,Clone)]
struct Network {
    /// Hardware address of network access point.
    mac: String,

    /// Name of network.
    ssid: String,

    /// Channel network is broadcast on.
    channel: String,
    
    /// Strength of network signal in dBm.
    strength: String,
    
    /// When the measurement was taken, unix time.
    time_scanned: u128,
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	   write!(f, "{ssid} ({mac}, {strength} dBm)", ssid=self.ssid, mac=self.mac, strength=self.strength)
    }
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
	   println!("New measurement properties:");
	   
	   // Prompt user for position
        let mut position = Coordinate::new();
	   
	   let mut get_pos_done = false;
        while !get_pos_done {
            print!("    Coordinates (x y z): ");
            stdout().flush().expect("failed to flush stdout");
            let mut pos_str = String::new();
            stdin().read_line(&mut pos_str).expect("failed to read input");
            pos_str = pos_str.replace("\n", "");

            let parts: Vec<&str> = pos_str.split(" ").collect();
            if parts.len() != 3 {
                println!("    Error: Must be in format \"x y z\"");
                continue;
            }

		  position.x = match parts[0].parse::<f32>() {
			 Ok(v) => v,
			 Err(e) => {
				println!("    Error: Failed to parse x as float: {}", e);
				continue;
			 },
		  };
		  position.y = match parts[1].parse::<f32>() {
			 Ok(v) => v,
			 Err(e) => {
				println!("    Error: Failed to parse y as float: {}", e);
				continue;
			 },
		  };
		  position.z = match parts[2].parse::<f32>() {
			 Ok(v) => v,
			 Err(e) => {
				println!("    Error: Failed to parse z as float: {}", e);
				continue;
			 },
		  };
		  
            get_pos_done = true;
        }

        // Prompt user for notes
        print!("    Notes (empty for none): ");

        let mut notes = String::new();
        stdout().flush().expect("failed to flush stdout");
        stdin().read_line(&mut notes).expect("failed to read input");
        notes = notes.replace("\n", "");

        // Scan networks
	   println!("Scanning");
	   
	   let mut networks = Network::scan()?;
	   networks.sort_by_key(|n| n.mac.clone());
	   
	   if networks.len() == 0 {
		  println!("Warning: No networks were found, this indicates that you may have to run this tool with elevated privileges");
	   }

	   let mut ssid_max_len = 0;
	   for network in &networks {
		  if network.ssid.len() > ssid_max_len {
			 ssid_max_len = network.ssid.len();
		  }
	   }
	   
	   let mut ssid_len_match = Vec::<Network>::new();
	   for network in &networks {
		  let mut matched = network.clone();

		  while matched.ssid.len() < ssid_max_len {
			 matched.ssid += " ";
		  }
		  
		  ssid_len_match.push(matched);
	   }

	   let networks_plural_str = match networks.len() != 1 {
		  true => "s",
		  false => "",
	   };

	   println!("Measured {} network{}:", networks.len(), networks_plural_str);

	   for network in &ssid_len_match {
		  println!("    {}", network);
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

impl fmt::Display for ScanMap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	   let notes_str = match self.notes.len() > 0 {
		  true => format!("{}, ", self.notes),
		  false => String::new(),
	   };
	   let node_plural_str = match self.nodes.len() > 0 {
		  true => "s",
		  false => "",
	   };
	   write!(f, "{name} Scan Map ({notes_str}{node_count} Node{node_plural_str})", name=self.name, notes_str=notes_str, node_count=self.nodes.len(), node_plural_str=node_plural_str)
    }
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

	   let networks_plural_str = match node.networks.len() != 1 {
		  true => "s",
		  false => "",
	   };
	   println!("Recorded a new measurement with {} network{}", node.networks.len(), networks_plural_str);
        
        self.nodes.push(node);

	   Ok(())
    }
}

/// Possible sub-commands.
enum SubCmd<'a> {
    /// Record wireless information.
    Record(&'a ArgMatches<'a>)
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
                    .about("Records a new scan to the map")
				.arg(Arg::with_name("loop")
					.short("l")
					.help("Loop and keep prompting for new recordings until the user kills the process")))
	   .get_matches();

    let map_file = arg_matches.value_of("map_file").unwrap();

    // Determine sub-command to run
    let mut subcmd: Option<SubCmd> = None;

    if let Some(c) = arg_matches.subcommand_matches("record") {
	   subcmd = Some(SubCmd::Record(c));
    }

    if subcmd.is_none() {
	   die("invalid sub-command");
    }

    // Initialize scan map
    let map_file_path = Path::new(map_file);
    let mut scan_map = match map_file_path.exists() {
	   true => {
		  // Read existing scan map file
		  let scan_map = ScanMap::read(map_file_path).expect("failed to load existing scan map");

		  println!("Loaded {} from \"{}\"", scan_map, map_file_path.display());
		  
		  scan_map
	   },
	   false => {
		  // Create new scan map
		  println!("Creating a new scan map in \"{}\"", map_file_path.display());
		  println!("New scan map properties:");
		  
		  let mut scan_map = ScanMap::new();

		  let mut get_name_done = false;
		  while !get_name_done {
			 print!("    Name: ");
			 stdout().flush().expect("failed to flush stdout");
			 stdin().read_line(&mut scan_map.name)
				.expect("failed to read input");
			 scan_map.name = scan_map.name.replace("\n", "");

			 if scan_map.name.len() > 0 {
				get_name_done = true;
			 } else {
				println!("    Error: Name cannot be empty");
 			 }
		  }

		  print!("    Notes (empty for none): ");
		  stdout().flush().expect("failed to flush stdout");
		  stdin().read_line(&mut scan_map.notes).expect("failed to read input");
		  scan_map.notes = scan_map.notes.replace("\n", "");

		  scan_map
	   },
    };

    // Run sub-command
    match subcmd.unwrap() {
	   SubCmd::Record(subcmd_args) => {
		  let mut done_recording = false;
		  while !done_recording {
			 // Acquire new reading
			 scan_map.acquire().expect("failed to acquire new reading");

			 // Save scan map
			 scan_map.write(map_file_path).expect("failed to save scan map");

			 if !subcmd_args.is_present("loop") {
				done_recording = true;
			 } else {
				println!("");
			 }
		  }
	   },
    };
}
