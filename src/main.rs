#![allow(dead_code)]
#![allow(unused_imports)]
use std::collections::btree_map::Range;
use std::vec;
use std::collections; // Need for BST and Queue
use rand; // Need for RNG and distributions

struct Station {
    name: String,
    customers: Vec<Customer>,
}

impl Station {

    fn new(new_name: String) -> Station {
        let new_vec = Vec::new();
        return Station {name: new_name, customers: new_vec};
    }

    fn add_customer(&mut self, new_cust: Customer) {
        self.customers.push(new_cust);
    }


}

struct Line { // NOTE: For the sake of this simulator, we assume the line has no braches
    name: String,
    stations: Vec<Station>,
}

impl Line {

    fn new(line_name: String, station_names: &[&str]) -> Line {
        let mut station_vec = Vec::new();
        for i in 0..station_names.len() {
            station_vec.push(Station::new(String::from(station_names[i])));
        }
        return Line {stations: station_vec, name: line_name};
    }

}

#[derive(Debug)]
struct Train {
    capacity: u8,
    assist_capacity: u8, // Priority seating
}

#[derive(Debug)]
struct Customer {
    sat: f32, // Time at which we arrive at the station
    tbt: f32, // Time at which we board the train
    tet: f32, // Time at which we left the train

    start_at: u8, // The station this customert arrived at
    end_at: u8, // The station this customer wants to reach
    assist: bool, // Does this customer need priority seating?
}

impl Customer {

    fn empty() -> Customer { // Used for testing
        return Customer {sat: 0.0, tbt: 0.0, tet: 0.0, start_at: 0, end_at: 0, assist: false};
    }

}

fn main() {
    // Main simulation loop
    
    // Initalize

    // Create Millenium Line
    let mil_line_station_names = ["VCC-Clark", "Commercial–Broadway", "Renfrew", "Rupert", "Gilmore", "Brentwood Town Centre",
                                            "Holdom", "Sperling–Burnaby Lake", "Lake City Way", "Production Way–University", "Lougheed Town Centre"];

    let mut millennium_line: Line = Line::new(String::from("Millennium Line"), &mil_line_station_names);

    for i in 0..millennium_line.stations.len() {
        millennium_line.stations[i].add_customer(Customer::empty());
        println!("{} with {} customer", millennium_line.stations[i].name, millennium_line.stations[i].customers.len());
    }

}
