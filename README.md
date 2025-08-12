# transitsimulator: A discrete event based transit simulator

I made this program for a class on discrete event simulation design and analysis. It was made to simulate a day of service of Vancouver's Millennium SkyTrain line with the goal of providing information on how different dispatch schedules for trains affect different statistics such as customer wait time and average train usage.

The final report and analysis of data for this project is available upon request.

## Usage
If you wish to see how it runs for yourself, it can be used as follows.
1) Build the program with `cargo`
`cargo build -r`
2) Navigate to ./target/release to get to the built executable
`cd target/release`
3) Run the program as follows
`./transitsimulator <seed> [constant|timebased|popbased|translink] <parameter>`
*(The seed provided is the seed used for the randomization of customer arrivals. The middle argument determines the system used to decide how to dispatch trains. And the final argument takes a parameter to fine-tune how said system functions)*

Once ran, you will be presented with information on train and customer related statistics as well as statistics on how long the simulation took to run.