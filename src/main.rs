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
use rand_distr::num_traits::clamp;
use std::env;
use std::time;


// Need for RNG and distributions
use rand_distr::{Distribution, Exp}; // Gives us Exp and Poisson distributions

// Want for optimized RNG algs
use rand_chacha::ChaCha8Core;
use rand_chacha::ChaCha8Rng;

//// HYPERPARAMETRS ////
const NUMBER_OF_TRAINS : u8 = 50;
const TRAIN_CAPACITY : f32 = 332.0; // Lowest capcity train (ICTS Mark I w/ four cars)
const TRAIN_ASSIST_CAPACITY : u8 = 10;
const TRAIN_STOP_TIME : f32 = 0.05;
const FIRST_CUSTOMER_ARRIVALS_AT : f32 = 10.0;

// USed to control what debug messages are printed during the simulation
// NOTE: All of these slow down the simulation when toggled on and clutter the terminal
const PRINT_TRAIN_INFO : bool = false;
const PRINT_ARRIVAL_INFO : bool = false;
const PRINT_CUSTOMER_INFO : bool = false;

const PRINT_FULL_TIMES : bool = false; // Does not slow down sim considerably

const SIMULATION_LENGTH : f32 = 1200.0; // NOTE: PRODUCTION LENGTH = 20 HOURS = 1200 MINUTES

// Used by trains for direction
const EASTWARD : i8 = 1;
const WESTWARD : i8 = -1;

enum DispatchTypes {
    Constant(f32), // Release one train every lambda minutes
    TimeBased(f32), // Release trains based on the time graph, offset by lambda
    PopBased(f32), // Release trains based on the waiting population, scaled by lambda
    TransLink(), // Use translink's system
}

struct Simulation { // Holds the Line and the list of trains on it
    line : Line,
    train_list : Vec<Train>,
    time_elapsed : f32,
    future_event_list : BinaryHeap<DiscreteEvent>,
    customer_iat : ChaCha8Rng,
    bookkeeping : Bookkeeper,
    dispatch_type : DispatchTypes,
    translink_sampler : Vec<f32>,
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

    fn translink_time_sampler(&self) -> f32 {
        // Given a time, what should the rate that we send out trains be
        // Based on translink's own stats: https://www.translink.ca/schedules-and-maps/skytrain
    
        // Make sure to clamp time to simulation time
        let mins_2_hours = clamp(self.time_elapsed, 0.0, SIMULATION_LENGTH) / 60.0;
        let bottom_hour : usize = mins_2_hours.floor() as usize;
    
        // Note: No linear interpolation is used. Customers want easy to understand iats and decimals are not that.
        return self.translink_sampler[bottom_hour];
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

struct Bookkeeper {
    // Used to track important statistics throughout our simulation
    total_customers: f32,
    total_customers_boarded: f32,
    total_customers_departed: f32,
    total_station_waiting_time: f32,
    max_station_waiting_time: f32,
    max_station_waiting_time_t: f32,
    total_trains: f32,
    average_train_util_percent: f32,
    max_train_util_percent: f32,
    time_train_full_percent: f32,
}

impl Bookkeeper {

    fn new() -> Bookkeeper {
        // Returns a BookKeeper class with all stats properly initalized
        return Bookkeeper {total_customers : 0.0, total_customers_boarded: 0.0, 
            total_customers_departed: 0.0, total_station_waiting_time: 0.0, 
            max_station_waiting_time: 0.0, max_station_waiting_time_t: 0.0, average_train_util_percent: 0.0, 
            total_trains : 0.0, max_train_util_percent: 0.0, time_train_full_percent: 0.0};
    }

    fn generate_report(&self, title : String) {
        // Prints a report made out of interal stats to the terminal
        println!("{}\n", title);
        println!("Customers:");
        print!("    TOTAL CUSTOMERS BOARDED / DEPARTED / GENERATED: {} / {} / {}\n", self.total_customers_boarded, self.total_customers_departed, self.total_customers);
        print!("    AVERAGE WAIT TIME: {}\n", self.total_station_waiting_time / self.total_customers_boarded);
        print!("    MAXIMUM WAIT TIME: {} @ minute {}\n", self.max_station_waiting_time, self.max_station_waiting_time_t);
        
        println!("\nService Rate:");
        print!("  THROUGHPUT (customers/hour): {}\n", self.total_customers_departed / 20.0); // Customers per hour

        println!("\nTrain Usage:");
        print!("    AVERAGE TRAIN UTILIZATION: {}\n", self.average_train_util_percent);
        print!("    MAXIMUM TRAIN UTILIZATION: {}\n", self.max_train_util_percent);
        print!("    AVERAGE TIME TRAINS ARE FULL FOR: {}\n", self.time_train_full_percent);
    }

}

struct Station {
    name: String,
    customers: Vec<Customer>,
    west_customers: VecDeque<Customer>,
    east_customers: VecDeque<Customer>,
    customer_iat: f32,
}

impl Station {

    fn new(new_name: String, iat: f32) -> Station {
        let new_vec = Vec::new();
        return Station {name: new_name, customers: new_vec, east_customers : VecDeque::new(), west_customers : VecDeque::new(), customer_iat: iat};
    }

    fn add_customer(&mut self, new_cust: Customer) {
        if new_cust.get_direction() == EASTWARD {
            self.east_customers.push_back(new_cust);
        } else {
            self.west_customers.push_back(new_cust);
        }
    }

    fn get_true_iat(&self, time : f32) -> f32 {
        // Returns an actual iat given our current minute

        // Table of the per-hour multiplier to iat of each station
        let hour_periods = vec![0.1, 0.3, 0.7, 0.8, 1.25, 1.5, 1.25, 1.0, 0.9, 1.0, 1.2, 1.4, 1.75, 1.4, 1.1, 0.9, 0.7, 0.5, 0.3, 0.1, 0.0];
        //let hour_periods = vec![1.0; 21];
        
        let mins_2_hours = time / 60.0;
        let normalized_inter_hour = (time % 60.0) / 60.0;
        let bottom_hour : usize = mins_2_hours.floor() as usize;

        // Linter interp
        let a = hour_periods[usize::from(bottom_hour)];
        let b  = hour_periods[usize::from(bottom_hour) + 1];
        let n = ((1.0 - normalized_inter_hour) * a ) + (normalized_inter_hour * b);
        return n * self.customer_iat;

   }
}

struct Line { // NOTE: For the sake of this simulator, we assume the line has no braches
    name: String,
    stations: Vec<Station>,
    inter_station_traveltimes: Vec<f32>,
    east_trains: VecDeque<usize>, // Used to store trains ready to start their journey east
    west_trains: VecDeque<usize>, // Used to store trains ready to start their journey west
}

impl Line {

    fn new(line_name: String, station_names: &[&str], station_traveltimes: Vec<f32>, iats : Vec<f32>) -> Line {
        let mut station_vec = Vec::new();
        for i in 0..station_names.len() {
            station_vec.push(Station::new(String::from(station_names[i]), iats[i]));
        }
        
        let east_trains: VecDeque<usize> = VecDeque::new();
        let west_trains: VecDeque<usize> = VecDeque::new();
        return Line {stations: station_vec, name: line_name, east_trains: east_trains, west_trains: west_trains, inter_station_traveltimes : station_traveltimes};
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
    capacity: f32,
    active: bool, // Wether or not this train is in our system or on standby
    at_station: usize, // Current station we are at (or are headed to)
    in_motion: bool, // If this train is between stations or not
    direction: i8,
    customer_list: Vec<Customer>,
    riding_customers: f32,
    percent_full_total: f32,
    percent_full_max: f32,
    percent_full_test_amount: f32,
    times_full : f32,
}

impl Train {

    fn new(new_id : u8) -> Train {
        return Train{id : new_id, capacity : TRAIN_CAPACITY, 
            active : false, at_station : 0, in_motion : false, direction : EASTWARD, customer_list: Vec::new(),
            percent_full_total: 0.0, percent_full_test_amount: 0.0, riding_customers: 0.0, percent_full_max : 0.0, times_full: 0.0};
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

    fn has_capacity(&self) -> bool {
        // Returns true if we have room left for passengers, false if we don't
        return self.capacity > self.riding_customers;
    }

    fn poll_usage(&mut self) -> bool{
        // Returns true if we have room left for passengers, false if we don't
        let cur_cap_percent = self.riding_customers / self.capacity;
        self.percent_full_total += cur_cap_percent;
        self.percent_full_test_amount += 1.0;
        if cur_cap_percent > self.percent_full_max {
            self.percent_full_max = cur_cap_percent;
        }
        if cur_cap_percent > 0.99 {
            self.times_full += 1.0;
            return true;
        } else {
            return false;
        }
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

    fn get_direction(&self) -> i8 {
        // Returns the direction this customer wants to go in (EASTWARD || WESTWARD)
        if self.start_at < self.end_at {
            return WESTWARD;
        } else {
            return EASTWARD;
        }
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

        // Remove any thing into
        if sim.train_list[train_id].customer_list[customer_index - 1].end_at == station_id {
            sim.train_list[train_id].customer_list.remove(customer_index - 1);
            sim.train_list[train_id].riding_customers -= 1.0;
            customer_count += 1;
            sim.bookkeeping.total_customers_departed += 1.0;
        }
        customer_index -= 1;
    }

    if customer_count > 0 && PRINT_CUSTOMER_INFO {
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
    let mut customer_count = 0;
    let customers_missed;
    let train_station = sim.train_list[train_id].at_station;

    let mut boarding_customer: Customer;

    // EASTWARD
    if sim.train_list[train_id].direction == EASTWARD {
        customers_missed = sim.line.stations[train_station].west_customers.len();
        
        while !sim.line.stations[train_station].east_customers.is_empty() && sim.train_list[train_id].has_capacity() {
            boarding_customer = sim.line.stations[train_station].east_customers.pop_front().expect("ERR: EMPTY EAST CUSTOMER LIST");
            boarding_customer.tbt = sim.time_elapsed;

            sim.bookkeeping.total_station_waiting_time += boarding_customer.tbt - boarding_customer.sat; // Total waiting time
            if boarding_customer.tbt - boarding_customer.sat > sim.bookkeeping.max_station_waiting_time { // Max wait time check
                sim.bookkeeping.max_station_waiting_time = boarding_customer.tbt - boarding_customer.sat;
                sim.bookkeeping.max_station_waiting_time_t = sim.time_elapsed;
            }
            
            sim.bookkeeping.total_customers_boarded += 1.0; // Amount of customers boarded

            sim.train_list[train_id].riding_customers += 1.0;
            sim.train_list[train_id].customer_list.push(boarding_customer);
            customer_count += 1;
        }
    } else {
        customers_missed = sim.line.stations[train_station].east_customers.len();

        while !sim.line.stations[train_station].west_customers.is_empty() && sim.train_list[train_id].has_capacity() {
            boarding_customer = sim.line.stations[train_station].west_customers.pop_front().expect("ERR: EMPTY EAST CUSTOMER LIST");
            boarding_customer.tbt = sim.time_elapsed;

            sim.bookkeeping.total_station_waiting_time += boarding_customer.tbt - boarding_customer.sat; // Total waiting time
            if boarding_customer.tbt - boarding_customer.sat > sim.bookkeeping.max_station_waiting_time { // Max wait time check
                sim.bookkeeping.max_station_waiting_time = boarding_customer.tbt - boarding_customer.sat;
                sim.bookkeeping.max_station_waiting_time_t = sim.time_elapsed;
            }
            
            sim.bookkeeping.total_customers_boarded += 1.0; // Amount of customer sboarded

            sim.train_list[train_id].riding_customers += 1.0;
            sim.train_list[train_id].customer_list.push(boarding_customer);
            customer_count += 1;
        }
    }

    if customer_count > 0 && PRINT_CUSTOMER_INFO {
        println!("{} -- Train {} picked up {} passengers from {}", sim.time_elapsed, train_id, customer_count, sim.line.id_to_name(train_station)); 
    }
    if customers_missed > 0 && PRINT_CUSTOMER_INFO {
        println!("{} -- Train {} missed {} passengers from {}", sim.time_elapsed, train_id, customers_missed, sim.line.id_to_name(train_station)); 
    }

    let train_travel_time: f32;
    if sim.train_list[train_id].direction == EASTWARD {train_travel_time = sim.line.inter_station_traveltimes[sim.train_list[train_id].at_station];}
    else {train_travel_time = sim.line.inter_station_traveltimes[sim.train_list[train_id].at_station - 1];}

    let is_full = sim.train_list[train_id].poll_usage();
    if is_full && PRINT_FULL_TIMES {
        println!("-- Train {} is full at time {}! --", train_id, sim.time_elapsed)
    }

    sim.train_list[train_id].leave_to(station_id);
    sim.add_event(EventTypes::TrainArrival(train_id, station_id), sim.time_elapsed + train_travel_time);
    
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
            sim.add_event(EventTypes::TrainArrival(train_id, 0), sim.time_elapsed + 0.5);
            if PRINT_TRAIN_INFO {println!("{} -- Train {} RELEASED going EAST", sim.time_elapsed, train_id);}
        } else {
            if PRINT_TRAIN_INFO {println!("{} -- UNABLE TO RELEASE TRAIN EASTWARD!", sim.time_elapsed);}
        }
    }

    else if direction == WESTWARD { // RELEASE ONTO LAST STATION
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
            sim.add_event(EventTypes::TrainArrival(train_id, sim.line.length() - 1), sim.time_elapsed + TRAIN_STOP_TIME);
            if PRINT_TRAIN_INFO {println!("{} -- Train {} RELEASED going WEST", sim.time_elapsed, train_id);}
        } else {
            if PRINT_TRAIN_INFO {println!("{} -- UNABLE TO RELEASE TRAIN WESTWARD!", sim.time_elapsed);}
        }
    }

    // Arrival types
    match sim.dispatch_type {
        DispatchTypes::Constant(lambda) => sim.add_event(EventTypes::TrainRelease(direction), sim.time_elapsed + lambda),
        DispatchTypes::TransLink() => sim.add_event(EventTypes::TrainRelease(direction), sim.time_elapsed + sim.translink_time_sampler()),
        _ => println!("Error: Somehow a dispatch type has not been defined..."),
    }

    return sim;
}


fn customer_arrival(mut sim : Simulation, station_id: usize) -> Simulation {
    // Has a customer arrive at the given station with a random destination station and 
    // Adds a new customer arrival event using the given RNG var in the sim object

    // Generate target station
    let target_gen = rand::distributions::Uniform::new(0, sim.line.length());
    let mut target_station = station_id;
    while station_id == target_station {
        target_station = target_gen.sample(&mut sim.customer_iat);
    }

    let new_customer = Customer {sat : sim.time_elapsed, tbt: 0.0, tet: 0.0, 
        start_at: station_id, end_at : target_station, assist : false};
    
    sim.line.stations[station_id].add_customer(new_customer);

    // Update bookkeeping
    sim.bookkeeping.total_customers += 1.0;

    // Query new customer arrival event
    let iat = rand_distr::Exp::new(sim.line.stations[station_id].get_true_iat(sim.time_elapsed)).unwrap();
    let new_iat = iat.sample(&mut sim.customer_iat);
    sim.add_event(EventTypes::CustomerArrival(station_id), sim.time_elapsed + new_iat);

    if PRINT_ARRIVAL_INFO {
        println!("{} -- Added customer to station {} (Goal: {})", sim.time_elapsed, sim.line.id_to_name(station_id), sim.line.id_to_name(target_station));
    }

    return sim
}

fn main() {
    // Main simulation loop


    // Get command line arguements
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("ERROR: Please provide the following arguements\n<seed> <constant|timebased|popbased|translink> <parameter>");
        return
    }

    let seed : u64 = args[1].parse().unwrap();

    let dispatch_type : DispatchTypes;
    let parameter : f32;
    if args.len() == 3 {
        // Default parameters
        parameter = match args[2].to_lowercase().as_str() {
            "constant" => 6.0, // How many times a second (default is six seconds)
            "timebased" => 0.0, // Offset added to time function (default is none)
            "popbased" => 1.0, // Multiplier to population function (default is none)
            _ => 0.0,
        };
        println!("Useing default parameter: {}", parameter);
        
    } else {
        parameter = args[3].parse().unwrap();
    }

    if args[2].to_lowercase() == "constant" {
        dispatch_type = DispatchTypes::Constant(parameter);
    } else if args[2].to_lowercase() == "timebased" {
        dispatch_type = DispatchTypes::TimeBased(parameter);
    } else if args[2].to_lowercase() == "popbased" {
        dispatch_type = DispatchTypes::PopBased(parameter);
    } else if args[2].to_lowercase() == "translink" {
        dispatch_type = DispatchTypes::TransLink();
    } else {
        println!("INVALID DISPATCH METHOD, USE ONE OF THE FOLLOWING: <constant|timebased|popbased|translink>");
        return
    }

    // Initalize
    // Create Millenium Line
    let m_line_station_names = ["VCC-Clark", "Commercial–Broadway", "Renfrew", "Rupert", "Gilmore", "Brentwood Town Centre",
                                            "Holdom", "Sperling–Burnaby Lake", "Lake City Way", "Production Way–University", "Lougheed Town Centre",
                                            "Burquitlam", "Moody Centre", "Inlet Centre", "Coquitlam Central", "Lincoln", "Lafarge Lake - Douglas"];
    let m_line_station_traveltimes: Vec<f32> = vec![1.0, 3.0, 1.0, 2.0, 2.0, 2.0, 2.0, 3.0, 2.0, 2.0, 3.0, 5.0, 2.0, 3.0, 2.0, 1.0];
    let m_line_station_iats = vec![2.325, 15.909, 2.842, 2.05, 2.850, 5.483, 2.225, 1.55, 0.825, 4.125, 9.625, 3.817, 1.933, 1.650, 4.033, 2.925, 1.883];

    let millennium_line: Line = Line::new(String::from("Millennium Line"), &m_line_station_names, m_line_station_traveltimes, m_line_station_iats);

    // Load Trains
    let mut train_list: Vec<Train> = Vec::new();
    for i in 0..NUMBER_OF_TRAINS {
        train_list.push(Train::new(i));
    }

    // FEL
    let future_event_list : BinaryHeap<DiscreteEvent> = BinaryHeap::new();
    let mut new_event : DiscreteEvent;
    
    // RNG streams (For CRN)
    let customer_arrival_rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);

    // Distribution for TransLinks dispact times by the hour
    // vec![0.1, 0.3, 0.7, 0.8, 1.1, 1.5, 1.1, 0.8, 0.7, 0.85, 1.2, 1.35, 1.6, 1.35, 1.1, 0.9, 0.7, 0.5, 0.3, 0.1, 0.0];
    let tl_ait_periods = 
       vec![8.0, 8.0, 6.0, 6.0, 6.0, 4.0, 6.0, 6.0, 6.0, 6.0, 6.0, 4.0, 3.0, 4.0, 6.0, 6.0, 8.0, 8.0, 8.0, 10.0, 10.0];

    // Create simulator object
    let mut sim : Simulation = Simulation {line : millennium_line, train_list : train_list, future_event_list : future_event_list, 
        time_elapsed : 0.0, customer_iat : customer_arrival_rng, bookkeeping : Bookkeeper::new(), dispatch_type: dispatch_type, translink_sampler : tl_ait_periods};

    // Add inital events
    // Train releases
    sim.add_event(EventTypes::TrainRelease(EASTWARD), 0.0);
    sim.add_event(EventTypes::TrainRelease(WESTWARD), 0.0);
    sim.add_event(EventTypes::Dummy(), SIMULATION_LENGTH);

    // CUstomer arrivals
    for i in 0..sim.line.length() {
        sim.add_event(EventTypes::CustomerArrival(i), FIRST_CUSTOMER_ARRIVALS_AT);
    }
    
    // Add trains to queues equally
    let mut dir: i8 = 1;
    for i in 0..sim.train_list.len() {
        if dir == EASTWARD {sim.line.east_trains.push_back(i);}
        else {sim.line.west_trains.push_back(i);}
        dir *= -1;
    }

    // START SIMULATION TIMER
    let timer = time::Instant::now();

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
    }

    // Stop timer
    let sim_realtime = timer.elapsed();
    println!("-- Time to execute: {:.2?} --", sim_realtime);

    // Generate and print report

    // Sim needs to do some extra work for train related stats
    let mut usage_perecnt : f32;
    let mut max_usage_percent : f32 = 0.0;
    let mut time_full_percent : f32;
    for i in 0..usize::from(NUMBER_OF_TRAINS) {
        usage_perecnt =  (sim.train_list[i].percent_full_total * 100.0) / sim.train_list[i].percent_full_test_amount;
        time_full_percent = (sim.train_list[i].times_full * 100.0) / sim.train_list[i].percent_full_test_amount;
        if PRINT_TRAIN_INFO {println!("Train {}: Usage percent {}% | Full percent {}%", i, usage_perecnt, time_full_percent);}
        sim.bookkeeping.total_trains += 1.0;
        sim.bookkeeping.average_train_util_percent += usage_perecnt;
        sim.bookkeeping.time_train_full_percent += time_full_percent;
        if max_usage_percent < (sim.train_list[i].percent_full_max * 100.0) {
            max_usage_percent = sim.train_list[i].percent_full_max * 100.0;
        }
    }

    sim.bookkeeping.max_train_util_percent = max_usage_percent;
    sim.bookkeeping.average_train_util_percent /= sim.bookkeeping.total_trains;
    sim.bookkeeping.time_train_full_percent /= sim.bookkeeping.total_trains;

    let title_string : String;
    
    title_string = match sim.dispatch_type {
        DispatchTypes::Constant(lambda) => format!("Results of simulation using a constant dispatch with lambda of {} (SEED = {})", lambda, seed).to_string(),
        DispatchTypes::TransLink() => format!("Results of simulation using TransLink's dispatch system (SEED = {})", seed).to_string(),
        _ => "Simulation ran with unimplemented dispatch type... You shouldn't be seeing this message!".to_string(),
    };

    // Prints the report
    sim.bookkeeping.generate_report(title_string);

    // Empty FEL
    sim.future_event_list.clear();

}
