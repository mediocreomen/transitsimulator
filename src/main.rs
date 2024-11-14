#![allow(dead_code)]
use std::vec;
use std::collections; // Need for BST and Queue
use rand; // Need for RNG and distributions


struct Station {
    customers: Vec<Customer>,
}

struct Line { // NOTE: For the sake of this simulator, we assume the line has no braches
    stations: Vec<Station>,
}

#[derive(Debug)]
struct Train {
    capacity: u8,
    assist_capacity: u8, // Priority seating
}

#[derive(Debug)]
struct Customer {
    station_arrival_time: f32, // Time at which we arrive at the station
    train_board_time: f32, // Time at which we board the train
    train_exit_time: f32, // Time at which we left the train

    start_station: u8, // The station this customert arrived at
    end_station: u8, // The station this customer wants to reach
    needs_assist: bool, // Does this customer need priority seating?
}

fn main() {
    println!("Hello, world!");
}
