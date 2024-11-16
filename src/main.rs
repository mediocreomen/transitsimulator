#![allow(dead_code)]
#![allow(unused_imports)]
use std::collections::btree_map::Range;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::vec;
use std::collections; // Need for BST and Queue
use rand; // Need for RNG and distributions

//// HYPERPARAMETRS ////
const NUMBER_OF_TRAINS : u8 = 20;
const TRAIN_CAPACITY : u8 = 100;
const TRAIN_ASSIST_CAPACITY : u8 = 10;
const SIMULATION_LENGTH : f32 = 10.0;

struct Simulation { // Holds the Line and the list of trains on it
    line : Line,
    train_list : Vec<Train>,
    time_elapsed : f32,
    future_event_list : BinaryHeap<DiscreteEvent>,
}

impl Simulation {

    fn pop_event(&mut self) -> DiscreteEvent {
        // Pop the next event from the event list
        let new_event = self.future_event_list.pop();
        match new_event {
            Some(evnt) => return evnt,
            None => panic!("FEQ is EMPTY! WERE ALL GONNA DIE!!!"),
        }
    }

    fn add_event(&mut self, event_type: EventTypes, time: f32) {
        // Add an event to the future event list
        self.future_event_list.push(DiscreteEvent{event : event_type, time: time});
    }
}


enum EventTypes {
    TrainArrival(usize, usize), // TRAIN ID, STATION ID
    TrainDeparture(usize, usize), // TRAIN ID, NEXT STATION ID
    Dummy(), // DOES NOTHING
}

struct DiscreteEvent {
    event : EventTypes,
    time : f32,
}

impl Ord for DiscreteEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        other.time.total_cmp(&self.time)
    }
}

impl PartialOrd for DiscreteEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for DiscreteEvent {}

impl PartialEq for DiscreteEvent {
    fn eq(&self, other: &Self) -> bool {
        (self.time) == (other.time)
    }
}

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

    fn add_cust_at(&mut self, new_cust: Customer, station_index: usize) {
        self.stations[station_index].customers.push(new_cust);
    }

}

#[derive(Debug)]
struct Train {
    id: u8,
    capacity: u8,
    assist_capacity: u8, // Priority seating
    active: bool, // Wether or not this train is in our system or on standby
    to_station: u8, // Current station we are at (or are headed to)
    in_motion: bool, // If this train is between stations or not
}

impl Train {

    fn new(new_id : u8) -> Train {
        return Train{id : new_id, capacity : TRAIN_CAPACITY, assist_capacity : TRAIN_ASSIST_CAPACITY, 
            active : false, to_station : 0, in_motion : false};
    }

    fn arrive_at(mut self, station_id : u8) {
        self.in_motion = false;
        self.to_station = station_id;
    }

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


//// Event Code //// 
fn dummy_event(mut sim : Simulation) -> Simulation {
    return sim;
}


fn main() {
    // Main simulation loop
    
    // Initalize
    // Create Millenium Line
    let m_line_station_names = ["VCC-Clark", "Commercial–Broadway", "Renfrew", "Rupert", "Gilmore", "Brentwood Town Centre",
                                            "Holdom", "Sperling–Burnaby Lake", "Lake City Way", "Production Way–University", "Lougheed Town Centre"];

    let millennium_line: Line = Line::new(String::from("Millennium Line"), &m_line_station_names);

    for i in 0..millennium_line.stations.len() {
        println!("{} with {} customer", millennium_line.stations[i].name, millennium_line.stations[i].customers.len());
    }

    // Load Trains
    let mut train_list: Vec<Train> = Vec::new();
    for i in 0..NUMBER_OF_TRAINS {
        train_list.push(Train::new(i));
    }

    // FEL
    let future_event_list : BinaryHeap<DiscreteEvent> = BinaryHeap::new();
    let mut new_event : DiscreteEvent;

    let mut sim : Simulation = Simulation {line : millennium_line, train_list : train_list, future_event_list : future_event_list, time_elapsed : 0.0};

    // Add first event
    sim.add_event(EventTypes::Dummy(), 0.0);
    sim.add_event(EventTypes::Dummy(), 2.1);
    sim.add_event(EventTypes::Dummy(), 1.1);

    // Event Loop
    while sim.time_elapsed < SIMULATION_LENGTH && !sim.future_event_list.is_empty()  {
        
        // Get next event
        new_event = sim.pop_event();

        // DO THE THING
        match new_event.event {
            EventTypes::Dummy() => sim = dummy_event(sim),
            EventTypes::TrainArrival(train_id, station_id) => (),
            EventTypes::TrainDeparture(train_id, station_id) => (),
        }
        println!("New Time {}", new_event.time);
        println!("Events in FEQ {}", sim.future_event_list.len());

        // Set new time
        sim.time_elapsed = new_event.time
    }

    // Empty FEL
    sim.future_event_list.clear();

}
