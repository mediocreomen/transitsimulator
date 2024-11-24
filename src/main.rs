#![allow(dead_code)]
#![allow(unused_imports)]
use std::collections::btree_map::Range;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::vec;
use std::collections; // Need for BST and Queue
use rand; 
use rand::rngs::ThreadRng;
use rand::SeedableRng;
use rand_chacha;


// Need for RNG and distributions
use rand_distr::{Distribution, Exp}; // Gives us Exp and Poisson distributions

// Want for optimized RNG algs
use rand_chacha::ChaCha8Core;
use rand_chacha::ChaCha8Rng;

//// HYPERPARAMETRS ////
const NUMBER_OF_TRAINS : u8 = 4;
const TRAIN_CAPACITY : u8 = 100;
const TRAIN_ASSIST_CAPACITY : u8 = 10;

const PRINT_TRAIN_INFO : bool = false;
const PRINT_ARRIVAL_INFO : bool = false;
const PRINT_CUSTOMER_INFO : bool = true;

const SEED : u64 = 1;

const SIMULATION_LENGTH : f32 = 18.0; // NOTE: PRODUCTION LENGTH = 1,320 MINUTES

const EASTWARD : i8 = 1;
const WESTWARD : i8 = -1;

struct Simulation { // Holds the Line and the list of trains on it
    line : Line,
    train_list : Vec<Train>,
    time_elapsed : f32,
    future_event_list : BinaryHeap<DiscreteEvent>,
    customer_iat : ChaCha8Rng,
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
    TrainRelease(i8), // TRAVEL DIRECTION
    CustomerArrival(usize), // STATION ID
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
    east_trains: VecDeque<usize>, // Used to store trains ready to start their journey east
    west_trains: VecDeque<usize>, // Used to store trains ready to start their journey west
}

impl Line {

    fn new(line_name: String, station_names: &[&str]) -> Line {
        let mut station_vec = Vec::new();
        for i in 0..station_names.len() {
            station_vec.push(Station::new(String::from(station_names[i])));
        }
        let east_trains: VecDeque<usize> = VecDeque::new();
        let west_trains: VecDeque<usize> = VecDeque::new();
        return Line {stations: station_vec, name: line_name, east_trains: east_trains, west_trains: west_trains};
    }

    fn add_cust_at(&mut self, new_cust: Customer, station_index: usize) {
        self.stations[station_index].customers.push(new_cust);
    }

    fn length(&self) -> usize {
        return self.stations.len();
    }

    fn id_to_name(&self, station_id: usize) -> &String {
        return &self.stations[station_id].name;
    }
    
    fn release_westward(&mut self) -> Option<usize> {
        // Take the next train index out of the westward train queue and return it
        return self.west_trains.pop_front();
    }

    fn release_eastward(&mut self) -> Option<usize> {
        // Take the next train index out of the eastward train queue and return it
        return self.east_trains.pop_front();
    }

}

#[derive(Debug)]
struct Train {
    id: u8,
    capacity: u8,
    assist_capacity: u8, // Priority seating
    active: bool, // Wether or not this train is in our system or on standby
    at_station: usize, // Current station we are at (or are headed to)
    in_motion: bool, // If this train is between stations or not
    direction: i8,
    customer_list: Vec<Customer>,
}

impl Train {

    fn new(new_id : u8) -> Train {
        return Train{id : new_id, capacity : TRAIN_CAPACITY, assist_capacity : TRAIN_ASSIST_CAPACITY, 
            active : false, at_station : 0, in_motion : false, direction : EASTWARD, customer_list: Vec::new()};
    }

    fn arrive_at(&mut self, station_id : usize) {
        self.in_motion = false;
        self.at_station = station_id;
    }

    fn leave_to(&mut self, station_id : usize) {
        self.in_motion = true;
        self.at_station = station_id;
    }

    fn switch_direction(&mut self) {
        // Switches the direction of the given train
        if self.direction == EASTWARD {
            self.direction = WESTWARD;
        } else {
            self.direction = EASTWARD;
        }

    }

    fn disable(&mut self) {
        // Used when we have reached the end of the line and have been adeed into a queue to be re-deployed
        self.in_motion = false;
        self.active = false;
    }

}

#[derive(Debug)]
struct Customer {
    sat: f32, // Time at which we arrive at the station
    tbt: f32, // Time at which we board the train
    tet: f32, // Time at which we left the train

    start_at: usize, // The station this customert arrived at
    end_at: usize, // The station this customer wants to reach
    assist: bool, // Does this customer need priority seating?
}

impl Customer {

    fn empty() -> Customer { // Used for testing
        return Customer {sat: 0.0, tbt: 0.0, tet: 0.0, start_at: 0, end_at: 0, assist: false};
    }

}


//// Event Code //// 
fn dummy_event( sim : Simulation) -> Simulation {
    return sim;
}

fn train_arrival(mut sim : Simulation, train_id: usize, station_id: usize) -> Simulation {
    
    if PRINT_TRAIN_INFO { println!("{} -- Train {} ARRIVAL at {}", sim.time_elapsed, train_id, sim.line.id_to_name(station_id)); }

    // TODO: PUT CUSTOMER DEPARTURE CODE WHERE WHEN THAT EXISTS!!!!
    // THIS CODE UNBOARDS ALL PASSENGERS CURRENTLY (REGARDLESS OF GOAL STATION)
    let mut customer_count = 0;
    let mut customer_index: usize = 0;
    if sim.train_list[train_id].customer_list.len() > 0 {customer_index = sim.train_list[train_id].customer_list.len();}

    while customer_index > 0 { // NOTE: We work back-to_front to avoid ordering issues with removal
        sim.train_list[train_id].customer_list.remove(customer_index - 1); // For now just let custromers get freed into the aether
        customer_index -= 1;
        customer_count += 1;
    }

    if customer_count > 0 && PRINT_CUSTOMER_INFO{
        println!("{} -- Train {} dropped off {} passengers at {}", sim.time_elapsed, train_id, customer_count, sim.line.id_to_name(station_id)); 
    }

    // Terminal station check
    if sim.train_list[train_id].at_station == 0 && sim.train_list[train_id].direction == WESTWARD {
        sim.train_list[train_id].disable();
        if PRINT_TRAIN_INFO { println!("{} -- Train {} REACHED TERMINAL STATION (WESTWARD)", sim.time_elapsed, train_id); }
        sim.line.east_trains.push_back(train_id);
        return sim;
    }
    else if sim.train_list[train_id].at_station == sim.line.length() - 1 && sim.train_list[train_id].direction == EASTWARD {
        sim.train_list[train_id].disable();
        if PRINT_TRAIN_INFO { println!("{} -- Train {} REACHED TERMINAL STATION (WESTWARD)", sim.time_elapsed, train_id); }
        sim.line.west_trains.push_back(train_id);
        return sim;
    }

    sim.train_list[train_id].arrive_at(station_id);
    
    if sim.train_list[train_id].direction == EASTWARD {
        sim.add_event(EventTypes::TrainDeparture(train_id, station_id + 1), sim.time_elapsed + 0.5);
    } else { // WESTWARD
        sim.add_event(EventTypes::TrainDeparture(train_id, station_id - 1), sim.time_elapsed + 0.5);
    }

    return sim;
}

fn train_departure(mut sim : Simulation, train_id: usize, station_id: usize) -> Simulation {
    
    if PRINT_TRAIN_INFO {println!("{} -- Train {} DEPARTURE to {}", sim.time_elapsed, train_id, sim.line.id_to_name(station_id)); }

    // TODO: Get customers to board train
    // THIS CURRENT CODE PUTS ALL CUSTOMERS IN STATION ON TRAIN
    let mut customer_count = 0;
    let mut customer_index: usize = 0;
    let train_station = sim.train_list[train_id].at_station;
    if sim.line.stations[train_station].customers.len() > 0 {customer_index = sim.line.stations[train_station].customers.len();}

    while customer_index > 0 { // NOTE: We work back-to_front to avoid ordering issues with removal
        sim.train_list[train_id].customer_list.push(sim.line.stations[train_station].customers.remove(customer_index - 1));
        customer_index -= 1;
        customer_count += 1;
    }

    if customer_count > 0 && PRINT_CUSTOMER_INFO {
        println!("{} -- Train {} picked up {} passengers from {}", sim.time_elapsed, train_id, customer_count, sim.line.id_to_name(train_station)); 
    }

    sim.train_list[train_id].leave_to(station_id);
    sim.add_event(EventTypes::TrainArrival(train_id, station_id), sim.time_elapsed + 1.0);
    
    return sim;
}

fn release_train(mut sim : Simulation, direction : i8) -> Simulation {
    // Puts a train on the tracks going the given direction

    let mut train_id = 0;

    if direction == EASTWARD { // RELEASE ONTO STATION 0
        let mut is_train = true;
        match sim.line.release_eastward() {
            Some(s) => train_id = s,
            None => is_train = false
        }

        if is_train {
            // Put train on first station
            sim.train_list[train_id].active = true;
            sim.train_list[train_id].direction = direction;
            sim.train_list[train_id].at_station = 0;
            sim.add_event(EventTypes::TrainArrival(train_id, 0), sim.time_elapsed + 1.0);
            if PRINT_TRAIN_INFO {println!("{} -- Train {} RELEASED going EAST", sim.time_elapsed, train_id);}
        } else {
            if PRINT_TRAIN_INFO {println!("{} -- UNABLE TO RELEASE TRAIN EASTWARD!", sim.time_elapsed);}
        }
    }

    else if direction == WESTWARD { // RELEASE ONTO STATION 0
        let mut is_train = true;
        match sim.line.release_westward() {
            Some(s) => train_id = s,
            None => is_train = false
        }

        if is_train {
            // Put train on last station
            sim.train_list[train_id].active = true;
            sim.train_list[train_id].direction = direction;
            sim.train_list[train_id].at_station = sim.line.length() - 1;
            sim.add_event(EventTypes::TrainArrival(train_id, sim.line.length() - 1), sim.time_elapsed + 1.0);
            if PRINT_TRAIN_INFO {println!("{} -- Train {} RELEASED going WEST", sim.time_elapsed, train_id);}
        } else {
            if PRINT_TRAIN_INFO {println!("{} -- UNABLE TO RELEASE TRAIN WESTWARD!", sim.time_elapsed);}
        }
    }
    
    // Add next train to queue
    let exp_dist = Exp::new(0.5).unwrap();
    if train_id < sim.train_list.len() - 1 { // Only send another train if we have more trains!
        sim.add_event(EventTypes::TrainRelease(direction), sim.time_elapsed + exp_dist.sample(&mut rand::thread_rng()));
    }

    return sim;
}


fn customer_arrival(mut sim : Simulation, station_id: usize) -> Simulation {
    // Has a customer arrive at the given station with a random destination station and 
    // Adds a new customer arrival event using the given RNG var in the sim object

    // Add new customer to station TODO: CHANGE BE ANB ACTUAL CUSTOMER
    let new_customer = Customer {sat : sim.time_elapsed, tbt: 0.0, tet: 0.0, 
        start_at: station_id, end_at : station_id, assist : false};
    
    sim.line.stations[station_id].add_customer(new_customer);

    // Query new customer arrival event
    let iat = rand_distr::Exp::new(2.0).unwrap();
    let new_iat = iat.sample(&mut sim.customer_iat);
    sim.add_event(EventTypes::CustomerArrival(station_id), sim.time_elapsed + new_iat);

    if PRINT_ARRIVAL_INFO {
        println!("{} -- Added customer to station {} with next one to arrive in {} minutes", sim.time_elapsed, sim.line.id_to_name(station_id), new_iat);
    }

    return sim
}


fn main() {
    // Main simulation loop
    
    // Initalize
    // Create Millenium Line
    let m_line_station_names = ["VCC-Clark", "Commercial–Broadway", "Renfrew", "Rupert", "Gilmore", "Brentwood Town Centre",
                                            "Holdom", "Sperling–Burnaby Lake", "Lake City Way", "Production Way–University", "Lougheed Town Centre"];

    let millennium_line: Line = Line::new(String::from("Millennium Line"), &m_line_station_names);

    // Load Trains
    let mut train_list: Vec<Train> = Vec::new();
    for i in 0..NUMBER_OF_TRAINS {
        train_list.push(Train::new(i));
    }

    // FEL
    let future_event_list : BinaryHeap<DiscreteEvent> = BinaryHeap::new();
    let mut new_event : DiscreteEvent;
    
    // RNG streams (For CRN)
    let customer_arrival_rng = rand_chacha::ChaCha8Rng::seed_from_u64(SEED);

    // Create simulator object
    let mut sim : Simulation = Simulation {line : millennium_line, train_list : train_list, future_event_list : future_event_list, time_elapsed : 0.0, customer_iat : customer_arrival_rng};

    // Add first (few) events
    sim.add_event(EventTypes::TrainRelease(EASTWARD), 0.0);
    sim.add_event(EventTypes::TrainRelease(WESTWARD), 0.0);
    sim.add_event(EventTypes::CustomerArrival(1), 0.0);
    
    // Add trains to queues equally
    let mut dir: i8 = 1;
    for i in 0..sim.train_list.len() {
        if dir == EASTWARD {sim.line.east_trains.push_back(i);}
        else {sim.line.west_trains.push_back(i);}
        dir *= -1;
    }

    // Event Loop
    while sim.time_elapsed < SIMULATION_LENGTH && !sim.future_event_list.is_empty()  {
        
        // Get next event
        new_event = sim.pop_event();

        // Set new time
        sim.time_elapsed = new_event.time;

        // DO THE THING
        match new_event.event {
            EventTypes::Dummy() => sim = dummy_event(sim),
            EventTypes::TrainArrival(train_id, station_id) => sim = train_arrival(sim, train_id, station_id),
            EventTypes::TrainDeparture(train_id, station_id) => sim = train_departure(sim, train_id, station_id),
            EventTypes::TrainRelease(dir) => sim = release_train(sim, dir),
            EventTypes::CustomerArrival(station_id) => sim = customer_arrival(sim, station_id),
            _ => sim = dummy_event(sim)
        }
        //println!("New Time {}", new_event.time);
        //println!("Events in FEQ {}", sim.future_event_list.len());
    }

    // Empty FEL
    sim.future_event_list.clear();

}
